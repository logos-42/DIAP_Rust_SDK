// DIAP Rust SDK - 签名PeerID模块
// 使用DID私钥签名PeerID，其他节点可通过公钥验证归属但不直接暴露PeerID

use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// 签名后的PeerID（隐私保护版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPeerID {
    /// PeerID的哈希（而非明文）
    pub peer_id_hash: Vec<u8>,
    
    /// 对PeerID的签名
    pub signature: Vec<u8>,
    
    /// 盲化因子（可选，用于进一步隐私保护）
    pub blinding_factor: Option<Vec<u8>>,
    
    /// 方法标识
    pub method: String,
}

/// 使用Ed25519私钥签名PeerID（隐私保护版本）
/// 只暴露PeerID的哈希和签名，不暴露明文
pub fn encrypt_peer_id(
    did_secret_key: &SigningKey,
    peer_id: &PeerId,
) -> Result<EncryptedPeerID> {
    // 1. 计算PeerID的SHA256哈希
    let peer_id_bytes = peer_id.to_bytes();
    let mut hasher = Sha256::new();
    hasher.update(&peer_id_bytes);
    hasher.update(b"DIAP_PEER_ID_V2");
    let peer_id_hash = hasher.finalize().to_vec();
    
    // 2. 对PeerID进行签名
    let signature = did_secret_key.sign(&peer_id_bytes);
    
    log::info!("✓ PeerID已签名（隐私保护）");
    log::debug!("  原始PeerID: {}", peer_id);
    log::debug!("  哈希长度: {} 字节", peer_id_hash.len());
    log::debug!("  签名长度: {} 字节", signature.to_bytes().len());
    
    Ok(EncryptedPeerID {
        peer_id_hash,
        signature: signature.to_bytes().to_vec(),
        blinding_factor: None,
        method: "Ed25519-Signature-V2".to_string(),
    })
}

/// 已废弃：使用verify_peer_id_signature代替
#[deprecated(note = "使用verify_peer_id_signature验证PeerID归属")]
pub fn decrypt_peer_id(
    _did_public_key: &VerifyingKey,
    _encrypted: &EncryptedPeerID,
) -> Result<PeerId> {
    Err(anyhow::anyhow!("已废弃，使用verify_peer_id_signature代替"))
}

/// 验证PeerID签名（其他节点验证归属）
/// 返回: 签名是否有效
pub fn verify_peer_id_signature(
    did_public_key: &VerifyingKey,
    encrypted: &EncryptedPeerID,
    claimed_peer_id: &PeerId,
) -> Result<bool> {
    // 1. 验证PeerID哈希是否匹配
    let peer_id_bytes = claimed_peer_id.to_bytes();
    let mut hasher = Sha256::new();
    hasher.update(&peer_id_bytes);
    hasher.update(b"DIAP_PEER_ID_V2");
    let computed_hash = hasher.finalize();
    
    if computed_hash.as_slice() != encrypted.peer_id_hash.as_slice() {
        log::warn!("PeerID哈希不匹配");
        return Ok(false);
    }
    
    // 2. 验证签名
    let signature = Signature::from_bytes(
        encrypted.signature.as_slice().try_into()
            .context("签名格式错误")?
    );
    
    match did_public_key.verify(&peer_id_bytes, &signature) {
        Ok(_) => {
            log::info!("✓ PeerID签名验证通过");
            Ok(true)
        }
        Err(_) => {
            log::warn!("PeerID签名验证失败");
            Ok(false)
        }
    }
}

/// 使用私钥解密PeerID（持有者重建明文）
/// 注意：这个函数用于持有私钥的用户恢复自己的PeerID
pub fn decrypt_peer_id_with_secret(
    _did_secret_key: &SigningKey,
    encrypted: &EncryptedPeerID,
) -> Result<PeerId> {
    // 在新的签名方案中，无法从哈希反推原始PeerID
    // 持有者应该本地存储PeerID，不依赖DID文档恢复
    log::warn!("新方案中无法从签名恢复PeerID，请本地存储");
    
    // 返回哈希作为提示（实际应用中应该从本地存储读取）
    Err(anyhow::anyhow!(
        "签名方案不支持恢复PeerID，哈希: {}",
        hex::encode(&encrypted.peer_id_hash)
    ))
}

/// 验证PeerID所有权（通过ZKP证明）
/// 这是资源节点使用的方法：验证用户确实持有对应的私钥和PeerID
pub fn verify_encrypted_peer_id_ownership(
    did_public_key: &VerifyingKey,
    encrypted: &EncryptedPeerID,
    claimed_peer_id: &PeerId,
) -> Result<bool> {
    log::info!("验证PeerID所有权（通过签名）");
    
    // 使用签名方案验证
    verify_peer_id_signature(did_public_key, encrypted, claimed_peer_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    
    #[test]
    fn test_sign_and_verify_peer_id() {
        // 生成Ed25519密钥对
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        
        // 生成libp2p PeerID
        let libp2p_keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // 签名
        let signed = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        
        // 验证
        let is_valid = verify_peer_id_signature(&verifying_key, &signed, &peer_id).unwrap();
        
        assert!(is_valid, "PeerID签名验证应该通过");
        println!("✓ 签名验证测试通过");
    }
    
    #[test]
    fn test_signature_with_wrong_peer_id() {
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        
        // 原始PeerID
        let libp2p_keypair1 = Keypair::generate_ed25519();
        let peer_id1 = PeerId::from(libp2p_keypair1.public());
        
        // 签名PeerID1
        let signed = encrypt_peer_id(&signing_key, &peer_id1).unwrap();
        
        // 尝试用PeerID2验证
        let libp2p_keypair2 = Keypair::generate_ed25519();
        let peer_id2 = PeerId::from(libp2p_keypair2.public());
        
        let is_valid = verify_peer_id_signature(&verifying_key, &signed, &peer_id2).unwrap();
        
        assert!(!is_valid, "使用错误的PeerID验证应该失败");
        println!("✓ 错误PeerID验证测试通过");
    }
    
    #[test]
    fn test_signature_determinism() {
        use rand::RngCore;
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        let libp2p_keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // 多次签名应产生相同的结果（Ed25519是确定性的）
        let signed1 = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        let signed2 = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        
        assert_eq!(signed1.signature, signed2.signature);
        assert_eq!(signed1.peer_id_hash, signed2.peer_id_hash);
        
        // 都能正确验证
        let valid1 = verify_peer_id_signature(&verifying_key, &signed1, &peer_id).unwrap();
        let valid2 = verify_peer_id_signature(&verifying_key, &signed2, &peer_id).unwrap();
        
        assert!(valid1);
        assert!(valid2);
        
        println!("✓ 签名确定性测试通过");
    }
}

