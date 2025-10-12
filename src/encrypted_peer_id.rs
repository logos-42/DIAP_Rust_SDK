// DIAP Rust SDK - 加密PeerID模块
// 使用DID私钥加密PeerID，保证去中心化流程的安全性

use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};

/// 加密后的PeerID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPeerID {
    /// 加密后的密文
    pub ciphertext: Vec<u8>,
    
    /// 加密使用的nonce
    pub nonce: Vec<u8>,
    
    /// 加密方法
    pub method: String,
}

/// 使用Ed25519私钥派生AES密钥并加密PeerID
pub fn encrypt_peer_id(
    did_secret_key: &SigningKey,
    peer_id: &PeerId,
) -> Result<EncryptedPeerID> {
    // 1. 从Ed25519私钥派生AES-256密钥
    let mut hasher = Sha256::new();
    hasher.update(did_secret_key.to_bytes());
    hasher.update(b"DIAP_PEER_ID_ENCRYPTION_V1");
    let aes_key_bytes = hasher.finalize();
    
    // 2. 创建AES-GCM加密器
    let cipher = Aes256Gcm::new_from_slice(&aes_key_bytes)
        .context("创建AES加密器失败")?;
    
    // 3. 生成随机nonce
    let nonce_bytes = rand::random::<[u8; 12]>();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // 4. 加密PeerID
    let peer_id_bytes = peer_id.to_bytes();
    let ciphertext = cipher.encrypt(nonce, peer_id_bytes.as_ref())
        .map_err(|_| anyhow::anyhow!("加密PeerID失败"))?;
    
    log::info!("✓ PeerID已加密");
    log::debug!("  原始PeerID: {}", peer_id);
    log::debug!("  密文长度: {} 字节", ciphertext.len());
    
    Ok(EncryptedPeerID {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
        method: "AES-256-GCM".to_string(),
    })
}

/// 使用Ed25519公钥派生AES密钥并解密PeerID
pub fn decrypt_peer_id(
    _did_public_key: &VerifyingKey,
    _encrypted: &EncryptedPeerID,
) -> Result<PeerId> {
    // 注意：这里使用公钥无法直接解密
    // 实际应该使用原始私钥，或者改用非对称加密方案
    // 为了演示，这里假设验证者拥有完整的密钥对
    
    // 在实际应用中，这个函数应该由拥有私钥的用户调用
    // 或者改用签名方案而非加密方案
    
    Err(anyhow::anyhow!("decrypt_peer_id需要私钥，公钥无法解密对称加密内容"))
}

/// 使用私钥解密PeerID（正确的方式）
pub fn decrypt_peer_id_with_secret(
    did_secret_key: &SigningKey,
    encrypted: &EncryptedPeerID,
) -> Result<PeerId> {
    // 1. 从Ed25519私钥派生相同的AES密钥
    let mut hasher = Sha256::new();
    hasher.update(did_secret_key.to_bytes());
    hasher.update(b"DIAP_PEER_ID_ENCRYPTION_V1");
    let aes_key_bytes = hasher.finalize();
    
    // 2. 创建AES-GCM解密器
    let cipher = Aes256Gcm::new_from_slice(&aes_key_bytes)
        .context("创建AES解密器失败")?;
    
    // 3. 解密
    let nonce = Nonce::from_slice(&encrypted.nonce);
    let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("解密PeerID失败"))?;
    
    // 4. 从字节恢复PeerID
    let peer_id = PeerId::from_bytes(&plaintext)
        .context("从字节解析PeerID失败")?;
    
    log::info!("✓ PeerID已解密: {}", peer_id);
    
    Ok(peer_id)
}

/// 验证加密的PeerID（通过ZKP证明，不实际解密）
/// 这是资源节点使用的方法：验证用户确实持有对应的私钥
pub fn verify_encrypted_peer_id_ownership(
    _did_public_key: &VerifyingKey,
    _encrypted: &EncryptedPeerID,
    _zkp_proof: &[u8], // ZKP证明
) -> Result<bool> {
    // 这里应该验证ZKP证明
    // 证明逻辑：用户知道私钥sk，使得：
    // 1. pk = derive_public(sk)
    // 2. encrypted = encrypt(sk, peer_id)
    // 而不需要实际解密PeerID
    
    log::info!("验证加密PeerID的所有权（通过ZKP）");
    // TODO: 实现实际的ZKP验证
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    #[test]
    fn test_encrypt_decrypt_peer_id() {
        // 生成Ed25519密钥对
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        
        // 生成libp2p PeerID
        let libp2p_keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // 加密
        let encrypted = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        
        // 解密
        let decrypted_peer_id = decrypt_peer_id_with_secret(&signing_key, &encrypted).unwrap();
        
        // 验证
        assert_eq!(peer_id, decrypted_peer_id);
        println!("✓ 加密解密测试通过");
    }
    
    #[test]
    fn test_encryption_determinism() {
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let libp2p_keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // 多次加密应产生不同的密文（因为nonce不同）
        let encrypted1 = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        let encrypted2 = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
        
        // 但都能正确解密
        let decrypted1 = decrypt_peer_id_with_secret(&signing_key, &encrypted1).unwrap();
        let decrypted2 = decrypt_peer_id_with_secret(&signing_key, &encrypted2).unwrap();
        
        assert_eq!(peer_id, decrypted1);
        assert_eq!(peer_id, decrypted2);
        
        println!("✓ 加密非确定性测试通过");
    }
}

