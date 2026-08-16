//! V1.5 凭据加解密模块
//!
//! 使用 AES-256-GCM 对 SSH 密码/私钥进行加解密。
//! 主密钥从环境变量 `MERIDIANOPS_SSH_MASTER_KEY` 读取（32 字节 hex 编码 = 64 字符）。
//! 若未设置则使用内置默认密钥（仅开发环境，启动时会 WARN）。
//!
//! 存储格式：`hex(nonce[12] || ciphertext+tag)` — 前端不接触明文。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

const DEFAULT_DEV_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// 获取主密钥（32 字节）
pub fn master_key() -> [u8; 32] {
    let hex_key = std::env::var("MERIDIANOPS_SSH_MASTER_KEY").unwrap_or_else(|_| {
        tracing::warn!(
            "MERIDIANOPS_SSH_MASTER_KEY 未设置，使用默认开发密钥（生产环境必须配置）"
        );
        DEFAULT_DEV_KEY.to_string()
    });
    let mut key = [0u8; 32];
    let decoded = hex::decode(&hex_key).unwrap_or_else(|_| {
        tracing::error!("MERIDIANOPS_SSH_MASTER_KEY hex 解码失败，回退默认密钥");
        hex::decode(DEFAULT_DEV_KEY).unwrap()
    });
    let len = decoded.len().min(32);
    key[..len].copy_from_slice(&decoded[..len]);
    key
}

/// 加密明文 → hex(nonce + ciphertext)
pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    let key = master_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::Init(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Encrypt(e.to_string()))?;

    // nonce(12) + ciphertext 拼接后 hex 编码
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(hex::encode(&combined))
}

/// 解密 hex(nonce + ciphertext) → 明文
pub fn decrypt(hex_str: &str) -> Result<String, CryptoError> {
    if hex_str.is_empty() {
        return Ok(String::new());
    }
    let combined = hex::decode(hex_str)
        .map_err(|e| CryptoError::Decrypt(format!("hex 解码失败: {}", e)))?;
    if combined.len() < 13 {
        return Err(CryptoError::Decrypt("密文过短".to_string()));
    }
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key = master_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::Init(e.to_string()))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError::Decrypt(format!("UTF-8 解码失败: {}", e)))
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("加密初始化失败: {0}")]
    Init(String),
    #[error("加密失败: {0}")]
    Encrypt(String),
    #[error("解密失败: {0}")]
    Decrypt(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = "my-secret-password-123!@#";
        let encrypted = encrypt(plaintext).unwrap();
        assert_ne!(plaintext, encrypted);
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_empty_string() {
        let encrypted = encrypt("").unwrap();
        assert_eq!(encrypted, "");
        let decrypted = decrypt("").unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_private_key_roundtrip() {
        let pem = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\n-----END OPENSSH PRIVATE KEY-----";
        let encrypted = encrypt(pem).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(pem, decrypted);
    }
}
