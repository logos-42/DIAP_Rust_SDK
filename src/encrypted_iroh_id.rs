// DIAP Rust SDK - 加密 Iroh ID 模块
// 复用与 PeerID 相同的方案：AES-256-GCM，对称密钥由 Ed25519 私钥派生；签名覆盖 (ciphertext || nonce)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedIrohId {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,     // 12 bytes
    pub signature: Vec<u8>, // Ed25519(signature over ciphertext||nonce)
    pub method: String,     // AES-256-GCM-Ed25519-V3
}

pub fn encrypt_iroh_id(did_secret_key: &SigningKey, iroh_id_bytes: &[u8]) -> Result<EncryptedIrohId> {
    // 1) 从 Ed25519 私钥派生 AES-256 key
    let aes_key = derive_aes_key_from_ed25519(did_secret_key);

    // 2) 生成 nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 3) 加密
    let cipher = Aes256Gcm::new(&aes_key.into());
    let ciphertext = cipher
        .encrypt(nonce, iroh_id_bytes)
        .map_err(|e| anyhow::anyhow!("AES-GCM 加密失败: {:?}", e))?;

    // 4) 签名 (ciphertext || nonce)
    let mut sig_data = Vec::with_capacity(ciphertext.len() + nonce_bytes.len());
    sig_data.extend_from_slice(&ciphertext);
    sig_data.extend_from_slice(&nonce_bytes);
    let signature = did_secret_key.sign(&sig_data);

    Ok(EncryptedIrohId {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
        signature: signature.to_bytes().to_vec(),
        method: "AES-256-GCM-Ed25519-V3".to_string(),
    })
}

pub fn decrypt_iroh_id_with_secret(did_secret_key: &SigningKey, enc: &EncryptedIrohId) -> Result<Vec<u8>> {
    // 1) 验签
    let mut sig_data = Vec::with_capacity(enc.ciphertext.len() + enc.nonce.len());
    sig_data.extend_from_slice(&enc.ciphertext);
    sig_data.extend_from_slice(&enc.nonce);

    let signature = Signature::from_bytes(
        enc.signature
            .as_slice()
            .try_into()
            .context("IrohId 签名格式错误")?,
    );
    let verifying_key: VerifyingKey = did_secret_key.verifying_key();
    verifying_key
        .verify(&sig_data, &signature)
        .context("IrohId 签名验证失败")?;

    // 2) 解密
    let aes_key = derive_aes_key_from_ed25519(did_secret_key);
    let cipher = Aes256Gcm::new(&aes_key.into());
    let nonce = Nonce::from_slice(&enc.nonce);
    let plaintext = cipher
        .decrypt(nonce, enc.ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("AES-GCM 解密失败: {:?}", e))?;

    Ok(plaintext)
}

fn derive_aes_key_from_ed25519(signing_key: &SigningKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.to_bytes());
    hasher.update(b"DIAP_AES_KEY_V3");
    let hash = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

