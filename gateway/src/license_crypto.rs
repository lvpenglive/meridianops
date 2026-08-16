//! License 激活码签名验签模块（V2）
//!
//! 激活码格式：`MK-<base64url(json_payload)>.<base64url(signature)>`
//!
//! payload 结构：
//! ```json
//! {
//!   "edition": "Enterprise",
//!   "customer": "XX银行",
//!   "expiresAt": "2027-08-16T23:59:59Z",
//!   "fingerprint": "abc123def456",
//!   "issuedAt": "2026-08-16T00:00:00Z",
//!   "validDays": 365
//! }
//! ```

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx:: MySqlPool;

/// 内置 Ed25519 公钥（对应私钥由 MeridianOps 官方持有，用于签发激活码）
const LICENSE_PUBLIC_KEY: [u8; 32] = [
    0x01, 0xda, 0x46, 0x5a, 0x4f, 0x0a, 0x2a, 0xce, 0xda, 0x7a, 0x64, 0x5d, 0x6c, 0x5c, 0x13, 0xf8,
    0xdd, 0xd6, 0x3b, 0x50, 0xb3, 0xba, 0x88, 0x8b, 0x6e, 0x23, 0xbf, 0x8c, 0xdf, 0x02, 0x39, 0x63,
];

/// 激活码前缀
const LICENSE_PREFIX: &str = "MK-";

/// License payload 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensePayload {
    /// 授权版本: Community / Enterprise / Ultimate
    pub edition: String,
    /// 客户名称
    pub customer: String,
    /// 到期时间 RFC3339，空字符串 = 永不到期
    #[serde(default)]
    pub expires_at: String,
    /// 机器指纹（SHA256 前 12 hex 字符），空字符串 = 不绑定机器
    #[serde(default)]
    pub fingerprint: String,
    /// 签发时间 RFC3339
    pub issued_at: String,
    /// 授权有效天数（0 = 永久，仅用于信息展示，实际限制以 expires_at 为准）
    #[serde(default)]
    pub valid_days: i64,
}

/// 验签结果
#[derive(Debug)]
pub struct VerifiedLicense {
    pub payload: LicensePayload,
    /// 激活码是否通过签名验证
    pub signature_valid: bool,
    /// 机器指纹是否匹配（true = 匹配或未绑定，false = 不匹配）
    pub fingerprint_match: bool,
}

/// 验证激活码签名 + 机器指纹。
///
/// 返回 `Ok(VerifiedLicense)` 表示格式正确且签名通过；
/// 返回 `Err(String)` 表示格式错误或验签失败。
pub fn verify_license(license_str: &str, current_fingerprint: &str) -> Result<VerifiedLicense, String> {
    // 1. 去除前缀
    let stripped = license_str.trim();
    let body = if let Some(rest) = stripped.strip_prefix(LICENSE_PREFIX) {
        rest
    } else {
        // 也允许不带前缀
        stripped
    };

    // 2. 分割 payload.signature
    let dot_pos = body.rfind('.').ok_or("激活码格式错误：缺少签名分隔符 '.'")?;
    let (payload_b64, sig_b64) = (&body[..dot_pos], &body[dot_pos + 1..]);

    // 3. 解码 payload
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| format!("激活码 payload 解码失败: {}", e))?;
    let payload: LicensePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("激活码 payload 解析失败: {}", e))?;

    // 4. 解码签名
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|e| format!("激活码签名解码失败: {}", e))?;
    if sig_bytes.len() != 64 {
        return Err(format!("签名长度错误: 期望 64 字节, 实际 {}", sig_bytes.len()));
    }
    let signature = Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());

    // 5. 验签
    let verifying_key = VerifyingKey::from_bytes(&LICENSE_PUBLIC_KEY)
        .map_err(|e| format!("公钥加载失败: {}", e))?;
    let is_valid = verifying_key
        .verify(&payload_bytes, &signature)
        .is_ok();

    if !is_valid {
        return Err("激活码签名验证失败：签名无效或密钥不匹配".to_string());
    }

    // 6. 机器指纹校验
    let fingerprint_match = payload.fingerprint.is_empty() || payload.fingerprint == current_fingerprint;

    Ok(VerifiedLicense {
        payload,
        signature_valid: true,
        fingerprint_match,
    })
}

/// 获取当前机器指纹。
///
/// 指纹 = SHA256(mysql_server_uuid + hostname)[:12]
///
/// 绑定 MySQL 实例和部署主机，防止激活码被复制到其他环境。
pub async fn get_machine_fingerprint(pool: &MySqlPool) -> String {
    // MySQL server_uuid
    let server_uuid: String = sqlx::query_scalar("SELECT @@server_uuid")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "unknown-db".to_string());

    // 主机名
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-host".to_string());

    // SHA256 取前 12 hex 字符
    let mut hasher = Sha256::new();
    hasher.update(server_uuid.as_bytes());
    hasher.update(b"|");
    hasher.update(hostname.as_bytes());
    let hash = hasher.finalize();
    hex_encode(&hash[..6]) // 6 bytes = 12 hex chars
}

/// 简单的 hex 编码
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// hostname crate 需要 作为依赖
// 但我们不想增加额外依赖，用 std 方式获取
mod hostname {
    pub fn get() -> std::io::Result<std::ffi::OsString> {
        // Windows: 使用 %COMPUTERNAME% 环境变量
        // Linux/macOS: 使用 hostname 命令
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("COMPUTERNAME")
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "COMPUTERNAME not set"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new("hostname")
                .output()
                .map(|o| std::ffi::OsString::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        }
    }
}
