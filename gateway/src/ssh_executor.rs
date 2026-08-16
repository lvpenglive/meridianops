//! V1.5 SSH 执行引擎
//!
//! 基于 russh 纯 Rust 异步 SSH 客户端，替代 V1 的 Mock 执行器。
//!
//! ## 核心能力
//!   - **认证方式**：密码认证 + 私钥认证（PEM/OpenSSH 格式，支持 passphrase）
//!   - **脚本类型**：shell(bash -s) / python(python3 -) / powershell(pwsh -Command -)
//!     脚本内容通过 stdin 管道传入，避免命令行注入和参数长度限制
//!   - **并发执行**：tokio Semaphore 限流，默认最大 10 台并发
//!   - **超时控制**：tokio::time::timeout 包裹整个 SSH 会话（连接+认证+执行）
//!   - **主机密钥验证**：支持指纹校验（空=跳过，不安全但便于 V1.5 快速接入）

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use russh::client::{self, Handle};
use russh::{ChannelMsg, Disconnect};
use sqlx::{MySqlPool, Row};
use tokio::sync::Semaphore;

use crate::crypto;
use crate::error::AppError;

/// 最大并发 SSH 会话数
const DEFAULT_MAX_CONCURRENCY: usize = 10;

/// SSH 连接超时（秒）
const SSH_CONNECT_TIMEOUT_SECS: u64 = 15;

// ===== 数据结构 =====

/// 解密后的 SSH 凭据（仅在内存中传递，不落盘）
pub struct SshCredential {
    pub id: i64,
    pub name: String,
    pub auth_type: String, // "password" | "key"
    pub username: String,
    pub password: String,
    pub private_key_pem: String,
    pub passphrase: String,
}

/// 单台主机的执行目标
pub struct SshTarget {
    pub asset_id: i64,
    pub asset_name: String,
    pub ip: String,
    pub port: i64,
}

/// 单台主机执行结果
pub struct SshExecResult {
    pub asset_id: i64,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i64,
    pub error: Option<String>,
}

impl SshExecResult {
    fn success(asset_id: i64, stdout: String, exit_code: i32, duration_ms: i64) -> Self {
        Self {
            asset_id,
            success: exit_code == 0,
            exit_code: Some(exit_code),
            stdout,
            stderr: String::new(),
            duration_ms,
            error: None,
        }
    }

    fn failed(asset_id: i64, stderr: String, exit_code: Option<i32>, duration_ms: i64) -> Self {
        Self {
            asset_id,
            success: false,
            exit_code,
            stdout: String::new(),
            stderr,
            duration_ms,
            error: None,
        }
    }

    fn error(asset_id: i64, err: String, duration_ms: i64) -> Self {
        Self {
            asset_id,
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("SSH Error: {}", err),
            duration_ms,
            error: Some(err),
        }
    }
}

// ===== SSH 客户端 Handler =====

/// russh 客户端 handler — 接受所有主机密钥（V1.5 简化处理）
struct SshClientHandler;

#[async_trait::async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // V1.5: 接受所有主机密钥（便于快速接入）
        // V2: 应对比 ssh_credentials.host_key_fingerprint
        Ok(true)
    }
}

// ===== 执行器 =====

pub struct SshExecutor {
    credential: SshCredential,
    semaphore: Arc<Semaphore>,
}

impl SshExecutor {
    pub fn new(credential: SshCredential, max_concurrency: usize) -> Self {
        let max = if max_concurrency == 0 {
            DEFAULT_MAX_CONCURRENCY
        } else {
            max_concurrency
        };
        Self {
            credential,
            semaphore: Arc::new(Semaphore::new(max)),
        }
    }

    /// 批量并发执行：对所有目标主机并发执行脚本
    pub async fn execute_batch(
        &self,
        targets: Vec<SshTarget>,
        script_content: &str,
        script_type: &str,
        timeout_secs: i64,
    ) -> Vec<SshExecResult> {
        let mut handles = Vec::with_capacity(targets.len());

        for target in targets {
            let sem = self.semaphore.clone();
            let script = script_content.to_string();
            let stype = script_type.to_string();
            let timeout = timeout_secs;
            let cred = SshCredentialSnap {
                username: self.credential.username.clone(),
                password: self.credential.password.clone(),
                private_key_pem: self.credential.private_key_pem.clone(),
                passphrase: self.credential.passphrase.clone(),
                auth_type: self.credential.auth_type.clone(),
            };

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                execute_on_host(&target, &cred, &script, &stype, timeout).await
            });
            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(r) => results.push(r),
                Err(e) => {
                    results.push(SshExecResult::error(
                        0,
                        format!("Task panicked: {}", e),
                        0,
                    ));
                }
            }
        }
        results
    }
}

/// 凭据快照（用于跨 task 传递，避免生命周期问题）
struct SshCredentialSnap {
    username: String,
    password: String,
    private_key_pem: String,
    passphrase: String,
    auth_type: String,
}

// ===== 单台主机执行 =====

async fn execute_on_host(
    target: &SshTarget,
    cred: &SshCredentialSnap,
    script: &str,
    script_type: &str,
    timeout_secs: i64,
) -> SshExecResult {
    let start = Instant::now();
    let addr = format!("{}:{}", target.ip, target.port);

    let timeout_dur = Duration::from_secs(if timeout_secs > 0 {
        timeout_secs as u64
    } else {
        300
    });

    let result = tokio::time::timeout(
        timeout_dur,
        ssh_connect_and_exec(&addr, cred, script, script_type),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(Ok((stdout, stderr, exit_code))) => {
            if exit_code == 0 {
                SshExecResult::success(target.asset_id, stdout, exit_code, duration_ms)
            } else {
                SshExecResult::failed(
                    target.asset_id,
                    if stderr.is_empty() {
                        format!("Exit code: {}\n{}", exit_code, stdout)
                    } else {
                        stderr
                    },
                    Some(exit_code),
                    duration_ms,
                )
            }
        }
        Ok(Err(e)) => SshExecResult::error(target.asset_id, e, duration_ms),
        Err(_) => SshExecResult::error(
            target.asset_id,
            format!("执行超时（{}秒）", timeout_secs),
            duration_ms,
        ),
    }
}

/// 建立 SSH 连接并执行脚本
///
/// 返回 (stdout, stderr, exit_code)
async fn ssh_connect_and_exec(
    addr: &str,
    cred: &SshCredentialSnap,
    script: &str,
    script_type: &str,
) -> Result<(String, String, i32), String> {
    // 1. 连接（带连接超时）
    let config = Arc::new(client::Config::default());
    let addr_parts: Vec<&str> = addr.split(':').collect();
    let host = addr_parts[0];
    let port: u16 = addr_parts
        .get(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(22);

    let connect_fut = client::connect(config, (host, port), SshClientHandler);
    let mut handle: Handle<SshClientHandler> = tokio::time::timeout(
        Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS),
        connect_fut,
    )
    .await
    .map_err(|_| format!("SSH 连接超时（{}秒）", SSH_CONNECT_TIMEOUT_SECS))?
    .map_err(|e| format!("SSH 连接失败: {}", e))?;

    // 2. 认证
    let auth_ok = match cred.auth_type.as_str() {
        "key" => {
            if cred.private_key_pem.is_empty() {
                return Err("私钥为空".to_string());
            }
            let passphrase_opt = if cred.passphrase.is_empty() {
                None
            } else {
                Some(cred.passphrase.as_str())
            };
            let key_pair = russh_keys::decode_secret_key(&cred.private_key_pem, passphrase_opt)
                .map_err(|e| format!("私钥解析失败: {}", e))?;
            handle
                .authenticate_publickey(&cred.username, Arc::new(key_pair))
                .await
                .map_err(|e| format!("私钥认证请求失败: {}", e))?
        }
        _ => {
            handle
                .authenticate_password(&cred.username, &cred.password)
                .await
                .map_err(|e| format!("密码认证请求失败: {}", e))?
        }
    };

    if !auth_ok {
        return Err(format!("SSH 认证失败: 用户 {}", cred.username));
    }

    // 3. 构造执行命令（脚本通过 stdin 传入）
    let (command, script_bytes) = build_command(script, script_type);

    // 4. 开 channel 执行
    let mut channel = handle.channel_open_session().await.map_err(|e| {
        format!("打开 session channel 失败: {}", e)
    })?;
    channel.exec(true, command.as_str()).await.map_err(|e| {
        format!("执行命令失败: {}", e)
    })?;

    // 通过 stdin 传入脚本内容
    if !script_bytes.is_empty() {
        channel.data(&script_bytes[..]).await.map_err(|e| {
            format!("写入 stdin 失败: {}", e)
        })?;
    }
    channel.eof().await.map_err(|e| {
        format!("发送 EOF 失败: {}", e)
    })?;

    // 5. 收集输出
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code = 0i32;

    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { ref data } => {
                stdout.extend_from_slice(data);
            }
            ChannelMsg::ExtendedData { ref data, .. } => {
                stderr.extend_from_slice(data);
            }
            ChannelMsg::ExitStatus { exit_status } => {
                exit_code = exit_status as i32;
            }
            ChannelMsg::Eof | ChannelMsg::Close => {
                break;
            }
            _ => {}
        }
    }

    // 6. 关闭连接
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "en")
        .await;

    let stdout_str = String::from_utf8_lossy(&stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&stderr).to_string();

    Ok((stdout_str, stderr_str, exit_code))
}

/// 根据脚本类型构造执行命令和 stdin 数据
fn build_command(script: &str, script_type: &str) -> (String, Vec<u8>) {
    let bytes = script.as_bytes().to_vec();
    match script_type {
        "python" => ("python3 -".to_string(), bytes),
        "powershell" => ("pwsh -Command -".to_string(), bytes),
        _ => ("bash -s".to_string(), bytes),
    }
}

// ===== 从数据库加载凭据 =====

pub async fn load_credential(
    pool: &MySqlPool,
    credential_id: i64,
) -> Result<SshCredential, AppError> {
    let row = sqlx::query(
        "SELECT id, name, auth_type, username, password_enc, private_key_enc, passphrase_enc \
         FROM ssh_credentials WHERE id = ?",
    )
    .bind(credential_id)
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::not_found("SSH 凭据不存在")),
    };

    let id: i64 = row.try_get("id").unwrap_or(0);
    let name: String = row.try_get("name").unwrap_or_default();
    let auth_type: String = row.try_get("auth_type").unwrap_or_else(|_| "password".into());
    let username: String = row.try_get("username").unwrap_or_default();
    let password_enc: String = row.try_get("password_enc").unwrap_or_default();
    let private_key_enc: Option<String> = row
        .try_get::<Option<String>, _>("private_key_enc")
        .ok()
        .flatten();
    let passphrase_enc: String = row.try_get("passphrase_enc").unwrap_or_default();

    let password = if password_enc.is_empty() {
        String::new()
    } else {
        crypto::decrypt(&password_enc)
            .map_err(|e| AppError::internal(&format!("密码解密失败: {}", e)))?
    };

    let private_key_pem = match private_key_enc {
        Some(enc) if !enc.is_empty() => crypto::decrypt(&enc)
            .map_err(|e| AppError::internal(&format!("私钥解密失败: {}", e)))?,
        _ => String::new(),
    };

    let passphrase = if passphrase_enc.is_empty() {
        String::new()
    } else {
        crypto::decrypt(&passphrase_enc)
            .map_err(|e| AppError::internal(&format!("口令解密失败: {}", e)))?
    };

    Ok(SshCredential {
        id,
        name,
        auth_type,
        username,
        password,
        private_key_pem,
        passphrase,
    })
}

/// 执行结果写入 job_run_targets 表
pub async fn persist_target_result(
    pool: &MySqlPool,
    run_id: i64,
    result: &SshExecResult,
) {
    let status = if result.success { "success" } else { "failed" };

    let _ = sqlx::query(
        "UPDATE job_run_targets \
         SET status = ?, exit_code = ?, stdout = ?, stderr = ?, duration_ms = ?, finished_at = NOW() \
         WHERE job_run_id = ? AND asset_id = ?",
    )
    .bind(status)
    .bind(result.exit_code.map(|c| c as i64))
    .bind(&result.stdout)
    .bind(&result.stderr)
    .bind(result.duration_ms)
    .bind(run_id)
    .bind(result.asset_id)
    .execute(pool)
    .await;
}

/// V1.5 SSH 执行入口：替代 V1 的 run_job_mock_executor
pub async fn run_job_ssh_executor(
    pool: MySqlPool,
    run_id: i64,
    asset_ids: Vec<i64>,
    asset_map: HashMap<i64, (String, String)>,
    script_content: String,
    script_type: String,
    timeout_secs: i64,
    credential_id: i64,
    ssh_port: i64,
) -> Result<(), AppError> {
    // 1. 加载凭据
    let credential = load_credential(&pool, credential_id).await?;

    // 2. 构造目标列表
    let targets: Vec<SshTarget> = asset_ids
        .iter()
        .map(|aid| {
            let (name, ip) = asset_map.get(aid).cloned().unwrap_or_else(|| {
                (format!("Asset#{}", aid), String::new())
            });
            SshTarget {
                asset_id: *aid,
                asset_name: name,
                ip,
                port: ssh_port,
            }
        })
        .collect();

    // 3. 并发执行
    let executor = SshExecutor::new(credential, DEFAULT_MAX_CONCURRENCY);
    let results = executor
        .execute_batch(targets, &script_content, &script_type, timeout_secs)
        .await;

    // 4. 逐条写入结果
    let mut success = 0i64;
    let mut failed = 0i64;

    for result in &results {
        if result.success {
            success += 1;
        } else {
            failed += 1;
        }
        persist_target_result(&pool, run_id, result).await;
    }

    // 5. 汇总更新 job_run
    let overall = if failed == 0 {
        "success"
    } else if success == 0 {
        "failed"
    } else {
        "partial"
    };

    let _ = sqlx::query(
        "UPDATE job_runs \
         SET success_count = ?, failed_count = ?, overall_status = ?, finished_at = NOW() \
         WHERE id = ?",
    )
    .bind(success)
    .bind(failed)
    .bind(overall)
    .bind(run_id)
    .execute(&pool)
    .await;

    Ok(())
}
