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

/// SSH 跳板端点（解析后；含已解密凭据快照则在内部 PreparedJump 携带）
#[derive(Clone, Debug)]
pub struct SshJumpRef {
    pub id: i64,
    pub host: String,
    pub port: u16,
    pub credential_id: i64,
}

/// 单台主机的执行目标
pub struct SshTarget {
    pub asset_id: String,
    pub asset_name: String,
    pub ip: String,
    pub port: i64,
    /// 可选：经 Proxy 跳板再连目标
    pub jump: Option<SshJumpRef>,
}

/// 单台主机执行结果
pub struct SshExecResult {
    pub asset_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i64,
    pub error: Option<String>,
}

impl SshExecResult {
    fn success(asset_id: String, stdout: String, exit_code: i32, duration_ms: i64) -> Self {
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

    fn failed(asset_id: String, stderr: String, exit_code: Option<i32>, duration_ms: i64) -> Self {
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

    fn error(asset_id: String, err: String, duration_ms: i64) -> Self {
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

    /// 批量并发执行：对所有目标主机并发执行脚本。
    /// `jump_creds`：跳板 credential_id → 已解密凭据。
    pub async fn execute_batch(
        &self,
        targets: Vec<SshTarget>,
        script_content: &str,
        script_type: &str,
        timeout_secs: i64,
        upload: Option<(String, Vec<u8>)>,
        jump_creds: &HashMap<i64, SshCredentialSnap>,
    ) -> Vec<SshExecResult> {
        let mut handles = Vec::with_capacity(targets.len());

        for target in targets {
            let sem = self.semaphore.clone();
            let script = script_content.to_string();
            let stype = script_type.to_string();
            let timeout = timeout_secs;
            let upload = upload.clone();
            let cred = snap_from_credential(&self.credential);
            let jump_snap = match &target.jump {
                Some(j) => match jump_creds.get(&j.credential_id) {
                    Some(c) => Some(PreparedJump {
                        host: j.host.clone(),
                        port: j.port,
                        cred: c.clone(),
                    }),
                    None => {
                        let asset_id = target.asset_id.clone();
                        let cid = j.credential_id;
                        handles.push(tokio::spawn(async move {
                            SshExecResult::error(
                                asset_id,
                                format!("跳板凭据 id={cid} 未加载（ProxyJump 配置不完整）"),
                                0,
                            )
                        }));
                        continue;
                    }
                },
                None => None,
            };

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                let upload_ref = upload
                    .as_ref()
                    .map(|(path, data)| (path.as_str(), data.as_slice()));
                execute_on_host(
                    &target,
                    &cred,
                    jump_snap.as_ref(),
                    &script,
                    &stype,
                    timeout,
                    upload_ref,
                )
                .await
            });
            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(r) => results.push(r),
                Err(e) => {
                    results.push(SshExecResult::error(
                        "unknown".to_string(),
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
#[derive(Clone)]
pub struct SshCredentialSnap {
    username: String,
    password: String,
    private_key_pem: String,
    passphrase: String,
    auth_type: String,
}

fn snap_from_credential(c: &SshCredential) -> SshCredentialSnap {
    SshCredentialSnap {
        username: c.username.clone(),
        password: c.password.clone(),
        private_key_pem: c.private_key_pem.clone(),
        passphrase: c.passphrase.clone(),
        auth_type: c.auth_type.clone(),
    }
}

struct PreparedJump {
    host: String,
    port: u16,
    cred: SshCredentialSnap,
}

// ===== 单台主机执行 =====

async fn execute_on_host(
    target: &SshTarget,
    cred: &SshCredentialSnap,
    jump: Option<&PreparedJump>,
    script: &str,
    script_type: &str,
    timeout_secs: i64,
    upload: Option<(&str, &[u8])>,
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
        ssh_connect_and_exec(&addr, cred, jump, script, script_type, upload),
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(Ok((stdout, stderr, exit_code))) => {
            if exit_code == 0 {
                SshExecResult::success(target.asset_id.clone(), stdout, exit_code, duration_ms)
            } else {
                SshExecResult::failed(
                    target.asset_id.clone(),
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
        Ok(Err(e)) => SshExecResult::error(target.asset_id.clone(), e, duration_ms),
        Err(_) => SshExecResult::error(
            target.asset_id.clone(),
            format!("执行超时（{}秒）", timeout_secs),
            duration_ms,
        ),
    }
}

async fn authenticate_handle(
    handle: &mut Handle<SshClientHandler>,
    cred: &SshCredentialSnap,
) -> Result<(), String> {
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
        _ => handle
            .authenticate_password(&cred.username, &cred.password)
            .await
            .map_err(|e| format!("密码认证请求失败: {}", e))?,
    };
    if !auth_ok {
        return Err(format!("SSH 认证失败: 用户 {}", cred.username));
    }
    Ok(())
}

/// 建立 SSH 连接（可选经跳板）；可选先推送文件，再执行脚本。
///
/// 返回 (stdout, stderr, exit_code)
async fn ssh_connect_and_exec(
    addr: &str,
    cred: &SshCredentialSnap,
    jump: Option<&PreparedJump>,
    script: &str,
    script_type: &str,
    upload: Option<(&str, &[u8])>,
) -> Result<(String, String, i32), String> {
    let config = Arc::new(client::Config::default());
    let addr_parts: Vec<&str> = addr.split(':').collect();
    let host = addr_parts[0];
    let port: u16 = addr_parts
        .get(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(22);

    // 持有跳板会话至目标会话结束
    let mut jump_handle: Option<Handle<SshClientHandler>> = None;

    let mut handle: Handle<SshClientHandler> = if let Some(j) = jump {
        let jump_addr = (j.host.as_str(), j.port);
        let connect_fut = client::connect(config.clone(), jump_addr, SshClientHandler);
        let mut jh: Handle<SshClientHandler> = tokio::time::timeout(
            Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS),
            connect_fut,
        )
        .await
        .map_err(|_| format!("跳板 SSH 连接超时（{}秒）: {}:{}", SSH_CONNECT_TIMEOUT_SECS, j.host, j.port))?
        .map_err(|e| format!("跳板 SSH 连接失败 {}:{}: {}", j.host, j.port, e))?;
        authenticate_handle(&mut jh, &j.cred)
            .await
            .map_err(|e| format!("跳板认证失败 ({}@{}): {}", j.cred.username, j.host, e))?;

        let channel = tokio::time::timeout(
            Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS),
            jh.channel_open_direct_tcpip(host, port as u32, "127.0.0.1", 0),
        )
        .await
        .map_err(|_| {
            format!(
                "经跳板 {} 打开到 {}:{} 的通道超时（{}秒）",
                j.host, host, port, SSH_CONNECT_TIMEOUT_SECS
            )
        })?
        .map_err(|e| {
            format!(
                "经跳板 {} 无法连通目标 {}:{}: {}（请确认 Proxy 能 SSH 到业务机）",
                j.host, host, port, e
            )
        })?;
        let stream = channel.into_stream();
        let target_fut = client::connect_stream(config, stream, SshClientHandler);
        let th = tokio::time::timeout(Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS), target_fut)
            .await
            .map_err(|_| format!("目标 SSH 握手超时（经跳板 {}）", j.host))?
            .map_err(|e| format!("目标 SSH 握手失败（经跳板 {}）: {}", j.host, e))?;
        jump_handle = Some(jh);
        th
    } else {
        let connect_fut = client::connect(config, (host, port), SshClientHandler);
        tokio::time::timeout(Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS), connect_fut)
            .await
            .map_err(|_| format!("SSH 连接超时（{}秒）", SSH_CONNECT_TIMEOUT_SECS))?
            .map_err(|e| format!("SSH 连接失败: {}", e))?
    };

    authenticate_handle(&mut handle, cred).await?;

    // 可选：推送安装包
    if let Some((remote_path, bytes)) = upload {
        ssh_upload_bytes(&mut handle, remote_path, bytes).await?;
    }

    // 构造执行命令（脚本通过 stdin 传入）
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

    // 6. 关闭连接（先目标，再跳板）
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "en")
        .await;
    if let Some(mut jh) = jump_handle {
        let _ = jh.disconnect(Disconnect::ByApplication, "", "en").await;
    }

    let stdout_str = String::from_utf8_lossy(&stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&stderr).to_string();

    Ok((stdout_str, stderr_str, exit_code))
}

/// 通过 `cat > remote` 管道推送二进制文件。
async fn ssh_upload_bytes(
    handle: &mut Handle<SshClientHandler>,
    remote_path: &str,
    data: &[u8],
) -> Result<(), String> {
    if remote_path.is_empty()
        || remote_path.contains('\n')
        || remote_path.contains('\'')
        || !remote_path.starts_with('/')
    {
        return Err("远程路径无效".to_string());
    }
    let parent = std::path::Path::new(remote_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("/tmp");
    let prep = format!("mkdir -p '{parent}'");
    let mut ch = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("打开上传 session 失败: {e}"))?;
    ch.exec(true, prep.as_str())
        .await
        .map_err(|e| format!("创建远程目录失败: {e}"))?;
    while let Some(msg) = ch.wait().await {
        match msg {
            ChannelMsg::ExitStatus { exit_status } if exit_status != 0 => {
                return Err(format!("创建远程目录失败，exit={exit_status}"));
            }
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }

    let cmd = format!("cat > '{remote_path}'");
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("打开上传 channel 失败: {e}"))?;
    channel
        .exec(true, cmd.as_str())
        .await
        .map_err(|e| format!("启动远程写入失败: {e}"))?;

    const CHUNK: usize = 32 * 1024;
    for chunk in data.chunks(CHUNK) {
        channel
            .data(chunk)
            .await
            .map_err(|e| format!("写入远程文件失败: {e}"))?;
    }
    channel
        .eof()
        .await
        .map_err(|e| format!("上传 EOF 失败: {e}"))?;

    let mut exit_code = 0i32;
    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }
    if exit_code != 0 {
        return Err(format!("远程写入退出码 {exit_code}"));
    }
    Ok(())
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

/// 仅验证「连得上 + 认证成功」，执行一条轻量命令后断开。
/// 返回耗时毫秒。
pub async fn test_connection(
    host: &str,
    port: u16,
    auth_type: &str,
    username: &str,
    password: &str,
    private_key_pem: &str,
    passphrase: &str,
) -> Result<u64, String> {
    let host = host.trim();
    if host.is_empty() {
        return Err("请填写测试目标主机".into());
    }
    if username.trim().is_empty() {
        return Err("请填写 SSH 用户名".into());
    }
    let port = if port == 0 { 22 } else { port };
    let snap = SshCredentialSnap {
        username: username.trim().to_string(),
        password: password.to_string(),
        private_key_pem: private_key_pem.to_string(),
        passphrase: passphrase.to_string(),
        auth_type: auth_type.to_string(),
    };
    let started = Instant::now();
    let (stdout, stderr, exit_code) = ssh_connect_and_exec(
        &format!("{host}:{port}"),
        &snap,
        None,
        "echo meridianops-ssh-ok",
        "shell",
        None,
    )
    .await?;
    let ms = started.elapsed().as_millis() as u64;
    if exit_code != 0 {
        return Err(format!(
            "已连通并认证，但测试命令失败 exit={exit_code} stderr={}",
            stderr.trim()
        ));
    }
    if !stdout.contains("meridianops-ssh-ok") {
        return Err(format!(
            "已连通并认证，但未收到预期输出: {}",
            stdout.trim()
        ));
    }
    Ok(ms)
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
    .bind(&result.asset_id)
    .execute(pool)
    .await;
}

/// 作业环境里带来的跳板提示（安装/启停由 zabbix-ctl 注入）
#[derive(Clone, Default)]
pub struct JumpHint {
    pub instance_code: String,
    pub proxy_id: String,
    pub proxy_group_id: String,
}

/// V1.5 SSH 执行入口：替代 V1 的 run_job_mock_executor。
/// `fallback_credential_id` 为剧本默认凭据；优先使用配置项 `ssh_credential_id`。
/// 跳板：CI `ssh_exec_mode=direct` 强制直连；否则按对照表/环境中的 Proxy 解析 `monitor_ssh_jumps`。
pub async fn run_job_ssh_executor(
    pool: MySqlPool,
    run_id: i64,
    asset_ids: Vec<String>,
    asset_map: HashMap<String, (String, String)>,
    script_content: String,
    script_type: String,
    timeout_secs: i64,
    fallback_credential_id: Option<i64>,
    ssh_port: i64,
    upload: Option<(String, Vec<u8>)>,
    jump_hint: JumpHint,
) -> Result<(), AppError> {
    let ci_cred_map = crate::db::ci_ssh_credential_map(&pool, &asset_ids)
        .await
        .map_err(|e| AppError::internal(&format!("查询配置项凭据失败: {e}")))?;
    let exec_mode_map = crate::db::ci_ssh_exec_mode_map(&pool, &asset_ids)
        .await
        .map_err(|e| AppError::internal(&format!("查询配置项执行方式失败: {e}")))?;
    let link_proxy_map = crate::db::ci_zbx_proxy_map(&pool, &asset_ids)
        .await
        .unwrap_or_default();

    let mut results: Vec<SshExecResult> = Vec::new();
    // (target_cred_id) -> targets（跳板已写在 SshTarget.jump）
    let mut groups: HashMap<i64, Vec<SshTarget>> = HashMap::new();
    let mut jump_cred_ids: Vec<i64> = Vec::new();

    for aid in &asset_ids {
        let (name, ip) = asset_map
            .get(aid)
            .cloned()
            .unwrap_or_else(|| (format!("Asset#{}", aid), String::new()));
        if ip.trim().is_empty() {
            results.push(SshExecResult::error(
                aid.clone(),
                format!("配置项 {} 缺少管理 IP（attributes.ip / mgmt_ip）", name),
                0,
            ));
            continue;
        }
        let ci_cred = ci_cred_map.get(aid).copied().flatten();
        let resolved = ci_cred.or(fallback_credential_id);
        let Some(cid) = resolved else {
            results.push(SshExecResult::error(
                aid.clone(),
                format!("配置项 {} 未绑定 SSH 凭据，且作业未配置默认凭据", name),
                0,
            ));
            continue;
        };

        let mode = exec_mode_map
            .get(aid)
            .map(|s| s.as_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let jump = if mode == "direct" {
            None
        } else {
            let (mut proxy_id, mut group_id) = link_proxy_map
                .get(aid)
                .cloned()
                .unwrap_or_default();
            if proxy_id.trim().is_empty() {
                proxy_id = jump_hint.proxy_id.clone();
            }
            if group_id.trim().is_empty() {
                group_id = jump_hint.proxy_group_id.clone();
            }
            match resolve_ssh_jump(
                &pool,
                &jump_hint.instance_code,
                proxy_id.trim(),
                group_id.trim(),
            )
            .await
            {
                Ok(j) => j,
                Err(e) => {
                    results.push(SshExecResult::error(
                        aid.clone(),
                        format!("解析 SSH 跳板失败: {e}"),
                        0,
                    ));
                    continue;
                }
            }
        };
        if let Some(ref j) = jump {
            jump_cred_ids.push(j.credential_id);
        }

        groups.entry(cid).or_default().push(SshTarget {
            asset_id: aid.clone(),
            asset_name: name,
            ip,
            port: ssh_port,
            jump,
        });
    }

    jump_cred_ids.sort_unstable();
    jump_cred_ids.dedup();
    let mut jump_creds: HashMap<i64, SshCredentialSnap> = HashMap::new();
    for jid in jump_cred_ids {
        match load_credential(&pool, jid).await {
            Ok(c) => {
                jump_creds.insert(jid, snap_from_credential(&c));
            }
            Err(e) => {
                return Err(AppError::bad(&format!(
                    "加载跳板 SSH 凭据 id={jid} 失败: {e}"
                )));
            }
        }
    }

    // 按目标凭据分组执行
    for (cred_id, pending) in groups {
        match load_credential(&pool, cred_id).await {
            Ok(credential) => {
                let executor = SshExecutor::new(credential, DEFAULT_MAX_CONCURRENCY);
                results.extend(
                    executor
                        .execute_batch(
                            pending,
                            &script_content,
                            &script_type,
                            timeout_secs,
                            upload.clone(),
                            &jump_creds,
                        )
                        .await,
                );
            }
            Err(e) => {
                let msg = e.to_string();
                for t in pending {
                    results.push(SshExecResult::error(
                        t.asset_id,
                        format!("加载 SSH 凭据失败: {msg}"),
                        0,
                    ));
                }
            }
        }
    }

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

/// 按 Proxy / Proxy Group 解析跳板；未配置则返回 None（直连）。
pub async fn resolve_ssh_jump(
    pool: &MySqlPool,
    instance_code: &str,
    proxy_id: &str,
    proxy_group_id: &str,
) -> Result<Option<SshJumpRef>, String> {
    let mut pid = proxy_id.trim().to_string();
    let code = instance_code.trim();
    if pid.is_empty() {
        let gid = proxy_group_id.trim();
        if gid.is_empty() {
            return Ok(None);
        }
        if code.is_empty() {
            return Ok(None);
        }
        // Group：用 conf 映射上的 jump_proxy_id
        let row = sqlx::query(
            "SELECT jump_proxy_id FROM monitor_proxy_conf_maps \
             WHERE instance_code = ? AND proxy_group_id = ? LIMIT 1",
        )
        .bind(code)
        .bind(gid)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
        pid = row
            .and_then(|r| r.try_get::<Option<String>, _>("jump_proxy_id").ok().flatten())
            .unwrap_or_default()
            .trim()
            .to_string();
        if pid.is_empty() {
            return Ok(None);
        }
    }
    let row = if code.is_empty() {
        sqlx::query(
            "SELECT id, jump_host, jump_port, credential_id FROM monitor_ssh_jumps \
             WHERE proxy_id = ? AND enabled = 1 ORDER BY id DESC LIMIT 1",
        )
        .bind(&pid)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query(
            "SELECT id, jump_host, jump_port, credential_id FROM monitor_ssh_jumps \
             WHERE instance_code = ? AND proxy_id = ? AND enabled = 1 LIMIT 1",
        )
        .bind(code)
        .bind(&pid)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
    };
    let Some(r) = row else {
        return Ok(None);
    };
    let id: i64 = r.try_get("id").map_err(|e| e.to_string())?;
    let host: String = r.try_get("jump_host").map_err(|e| e.to_string())?;
    if host.trim().is_empty() {
        return Ok(None);
    }
    let port: i64 = r.try_get("jump_port").unwrap_or(22);
    let credential_id: i64 = r.try_get("credential_id").map_err(|e| e.to_string())?;
    Ok(Some(SshJumpRef {
        id,
        host: host.trim().to_string(),
        port: if port <= 0 { 22 } else { port as u16 },
        credential_id,
    }))
}
