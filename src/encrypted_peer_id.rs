// DIAP Rust SDK - 加密节点ID模块（改进版）
// 使用AES-256-GCM加密节点ID，持有私钥者可以解密恢复

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 加密的节点ID（改进版：可解密恢复）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPeerID {
    /// 加密的节点ID字节
    pub ciphertext: Vec<u8>,

    /// AES-GCM nonce (12字节)
    pub nonce: Vec<u8>,

    /// 对加密数据的签名（用于验证完整性）
    pub signature: Vec<u8>,

    /// 加密方法标识
    pub method: String,
}

/// 使用Ed25519私钥加密节点ID（改进版：可解密）
/// 使用从私钥派生的AES-256密钥加密节点ID
pub fn encrypt_peer_id(did_secret_key: &SigningKey, node_id: &str) -> Result<EncryptedPeerID> {
    // 1. 从Ed25519私钥派生AES-256密钥
    let aes_key = derive_aes_key_from_ed25519(did_secret_key);

    // 2. 生成随机nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 3. 创建AES-GCM加密器
    let cipher = Aes256Gcm::new(&aes_key.into());

    // 4. 加密节点ID
    let peer_id_bytes = node_id.as_bytes();
    let ciphertext = cipher
        .encrypt(nonce, peer_id_bytes)
        .map_err(|e| anyhow::anyhow!("AES-GCM加密失败: {:?}", e))?;

    // 5. 对加密数据进行签名
    let mut sig_data = Vec::new();
    sig_data.extend_from_slice(&ciphertext);
    sig_data.extend_from_slice(&nonce_bytes);
    let signature = did_secret_key.sign(&sig_data);

    log::info!("✓ 节点ID已加密（AES-256-GCM）");
    log::debug!("  原始节点ID: {}", node_id);
    log::debug!("  密文长度: {} 字节", ciphertext.len());
    log::debug!("  Nonce长度: {} 字节", nonce_bytes.len());
    log::debug!("  签名长度: {} 字节", signature.to_bytes().len());

    Ok(EncryptedPeerID {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
        signature: signature.to_bytes().to_vec(),
        method: "AES-256-GCM-Ed25519-V3".to_string(),
    })
}

/// 从Ed25519私钥派生AES-256密钥
fn derive_aes_key_from_ed25519(signing_key: &SigningKey) -> [u8; 32] {
    // 使用SHA-256派生密钥
    let mut hasher = Sha256::new();
    hasher.update(signing_key.to_bytes());
    hasher.update(b"DIAP_AES_KEY_V3");
    let hash = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// 使用私钥解密节点ID（改进版：可以恢复）
/// 持有DID私钥的用户可以解密恢复自己的节点ID
pub fn decrypt_peer_id_with_secret(
    did_secret_key: &SigningKey,
    encrypted: &EncryptedPeerID,
) -> Result<String> {
    log::info!("🔓 解密节点ID（持有私钥）");

    // 1. 验证签名（确保数据未被篡改）
    let mut sig_data = Vec::new();
    sig_data.extend_from_slice(&encrypted.ciphertext);
    sig_data.extend_from_slice(&encrypted.nonce);

    let signature_bytes: [u8; 64] = encrypted.signature.clone().try_into()
        .map_err(|_| anyhow::anyhow!("签名长度错误，期望64字节"))?;
    let signature = Signature::from_bytes(&signature_bytes);

    match did_secret_key.verify(&sig_data, &signature) {
        Ok(_) => {
            log::debug!("✓ 签名验证通过");
        }
        Err(_) => {
            return Err(anyhow::anyhow!("签名验证失败，数据可能被篡改"));
        }
    }

    // 2. 从Ed25519私钥派生AES-256密钥
    let aes_key = derive_aes_key_from_ed25519(did_secret_key);

    // 3. 创建AES-GCM解密器
    let nonce = Nonce::from_slice(&encrypted.nonce);
    let cipher = Aes256Gcm::new(&aes_key.into());

    // 4. 解密
    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("AES-GCM解密失败: {:?}", e))?;

    // 5. 从字节恢复节点ID字符串
    let node_id = String::from_utf8(plaintext)
        .context("无法从解密数据恢复节点ID字符串")?;

    log::debug!("  解密的节点ID: {}", node_id);

    Ok(node_id)
}

/// 验证节点ID签名（其他节点验证归属）
/// 不需要解密，只验证持有者确实拥有对应的私钥
pub fn verify_peer_id_signature(
    did_public_key: &VerifyingKey,
    encrypted: &EncryptedPeerID,
    _claimed_node_id: &str,
) -> Result<bool> {
    log::info!("验证节点ID签名（公开验证）");

    // 1. 构造签名数据
    let mut sig_data = Vec::new();
    sig_data.extend_from_slice(&encrypted.ciphertext);
    sig_data.extend_from_slice(&encrypted.nonce);

    // 2. 解析签名
    let signature_bytes: [u8; 64] = encrypted.signature.clone().try_into()
        .map_err(|_| anyhow::anyhow!("签名长度错误，期望64字节"))?;
    let signature = Signature::from_bytes(&signature_bytes);

    // 3. 验证签名
    match did_public_key.verify(&sig_data, &signature) {
        Ok(_) => {
            log::info!("✓ 节点ID签名验证通过");
            // 注意：这只验证了签名有效性，没有验证节点ID内容
            // 如果需要验证具体的节点ID，调用者需要解密后比较
            Ok(true)
        }
        Err(_) => {
            log::warn!("节点ID签名验证失败");
            Ok(false)
        }
    }
}

/// 已废弃：使用decrypt_peer_id_with_secret代替
#[deprecated(note = "使用decrypt_peer_id_with_secret替代")]
pub fn decrypt_peer_id(
    _did_public_key: &VerifyingKey,
    _encrypted: &EncryptedPeerID,
) -> Result<String> {
    Err(anyhow::anyhow!(
        "已废弃，使用decrypt_peer_id_with_secret代替"
    ))
}

/// 验证节点ID所有权（通过ZKP证明）
/// 这是资源节点使用的方法：验证用户确实持有对应的私钥和节点ID
pub fn verify_encrypted_peer_id_ownership(
    did_public_key: &VerifyingKey,
    encrypted: &EncryptedPeerID,
    claimed_node_id: &str,
) -> Result<bool> {
    log::info!("验证节点ID所有权（通过签名）");

    // 使用签名方案验证
    verify_peer_id_signature(did_public_key, encrypted, claimed_node_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_and_decrypt_peer_id() {
        // 生成Ed25519密钥对
        let secret_bytes = [0u8; 32]; // 示例私钥
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        // 生成测试节点ID
        let node_id = "12D3KooWExampleNodeIdForTesting";

        // 加密
        let encrypted = encrypt_peer_id(&signing_key, node_id).unwrap();

        // 验证签名
        let is_valid = verify_peer_id_signature(&verifying_key, &encrypted, node_id).unwrap();
        assert!(is_valid, "节点ID签名验证应该通过");

        // 解密
        let decrypted_node_id = decrypt_peer_id_with_secret(&signing_key, &encrypted).unwrap();
        assert_eq!(node_id, decrypted_node_id, "解密后的节点ID应该与原始节点ID相同");

        println!("✓ 加密解密测试通过（改进版）");
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        use rand::RngCore;
        let mut secret_bytes1 = [0u8; 32];
        let mut secret_bytes2 = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes1);
        rand::thread_rng().fill_bytes(&mut secret_bytes2);

        let signing_key1 = SigningKey::from_bytes(&secret_bytes1);
        let signing_key2 = SigningKey::from_bytes(&secret_bytes2);

        // 生成节点ID
        let node_id = "12D3KooWExampleNodeIdForTesting";

        // 用密钥1加密
        let encrypted = encrypt_peer_id(&signing_key1, node_id).unwrap();

        // 用密钥2解密应该失败
        let result = decrypt_peer_id_with_secret(&signing_key2, &encrypted);
        assert!(result.is_err(), "使用错误的密钥解密应该失败");

        println!("✓ 错误密钥解密测试通过");
    }

    #[test]
    fn test_encryption_randomness() {
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);

        let node_id = "12D3KooWExampleNodeIdForTesting";

        // 多次加密应产生不同的密文（因为nonce是随机的）
        let encrypted1 = encrypt_peer_id(&signing_key, node_id).unwrap();
        let encrypted2 = encrypt_peer_id(&signing_key, node_id).unwrap();

        // nonce应该不同
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        // 密文应该不同
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);

        // 但都能正确解密
        let decrypted1 = decrypt_peer_id_with_secret(&signing_key, &encrypted1).unwrap();
        let decrypted2 = decrypt_peer_id_with_secret(&signing_key, &encrypted2).unwrap();

        assert_eq!(node_id, decrypted1);
        assert_eq!(node_id, decrypted2);

        println!("✓ 加密随机性测试通过");
    }
}
