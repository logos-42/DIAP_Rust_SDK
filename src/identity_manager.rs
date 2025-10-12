// DIAP Rust SDK - 统一身份管理模块（ZKP版本）
// 使用ZKP验证DID-CID绑定，无需IPNS

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::key_manager::KeyPair;
use crate::did_builder::{DIDBuilder, DIDDocument, get_did_document_from_cid};
use crate::ipfs_client::IpfsClient;
use crate::zkp_prover::{ZKPProver, ZKPVerifier, ProofResult};
use crate::encrypted_peer_id::{EncryptedPeerID, decrypt_peer_id_with_secret};
use libp2p::PeerId;
use ed25519_dalek::SigningKey;
use base64::{Engine as _, engine::general_purpose};

/// 智能体信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// 智能体名称
    pub name: String,
    
    /// 服务端点列表
    pub services: Vec<ServiceInfo>,
    
    /// 描述信息（可选）
    pub description: Option<String>,
    
    /// 标签（可选）
    pub tags: Option<Vec<String>>,
}

/// 服务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// 服务类型
    pub service_type: String,
    
    /// 服务端点
    pub endpoint: serde_json::Value,
}

/// 身份注册结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistration {
    /// DID标识符（did:key格式）
    pub did: String,
    
    /// IPFS CID（DID文档的内容地址）
    pub cid: String,
    
    /// DID文档
    pub did_document: DIDDocument,
    
    /// 加密的PeerID
    pub encrypted_peer_id_hex: String,
    
    /// 注册时间
    pub registered_at: String,
}

/// 身份验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityVerification {
    /// DID标识符
    pub did: String,
    
    /// CID
    pub cid: String,
    
    /// ZKP验证状态
    pub zkp_verified: bool,
    
    /// 验证详情
    pub verification_details: Vec<String>,
    
    /// 验证时间
    pub verified_at: String,
}

/// 统一身份管理器（ZKP版本）
pub struct IdentityManager {
    /// IPFS客户端
    ipfs_client: IpfsClient,
    
    /// ZKP证明生成器
    zkp_prover: ZKPProver,
    
    /// ZKP验证器
    zkp_verifier: ZKPVerifier,
}

impl IdentityManager {
    /// 创建新的身份管理器
    pub fn new(ipfs_client: IpfsClient) -> Self {
        Self {
            ipfs_client,
            zkp_prover: ZKPProver::new(),
            zkp_verifier: ZKPVerifier::new(),
        }
    }
    
    /// 📝 注册身份（简化流程：一次上传 + ZKP绑定）
    pub async fn register_identity(
        &self,
        agent_info: &AgentInfo,
        keypair: &KeyPair,
        libp2p_peer_id: &PeerId,
    ) -> Result<IdentityRegistration> {
        log::info!("🚀 开始身份注册流程（ZKP版本）");
        log::info!("  智能体: {}", agent_info.name);
        log::info!("  DID: {}", keypair.did);
        log::info!("  PeerID: {}", libp2p_peer_id);
        
        // 步骤1: 创建DID构建器并添加服务端点
        let mut builder = DIDBuilder::new(self.ipfs_client.clone());
        
        for service in &agent_info.services {
            builder.add_service(&service.service_type, service.endpoint.clone());
        }
        
        // 步骤2: 创建并发布DID文档（单次上传）
        let publish_result = builder.create_and_publish(keypair, libp2p_peer_id).await
            .context("DID发布失败")?;
        
        log::info!("✅ 身份注册成功");
        log::info!("  DID: {}", publish_result.did);
        log::info!("  CID: {}", publish_result.cid);
        
        Ok(IdentityRegistration {
            did: publish_result.did,
            cid: publish_result.cid,
            did_document: publish_result.did_document,
            encrypted_peer_id_hex: hex::encode(&publish_result.encrypted_peer_id.ciphertext),
            registered_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 🔐 生成DID-CID绑定的ZKP证明
    pub fn generate_binding_proof(
        &self,
        keypair: &KeyPair,
        did_document: &DIDDocument,
        _cid: &str,
        nonce: &[u8],
    ) -> Result<ProofResult> {
        log::info!("🔐 生成DID-CID绑定证明");
        
        // 计算DID文档的哈希
        use blake2::{Blake2s256, Digest};
        let did_json = serde_json::to_string(did_document)?;
        let hash = Blake2s256::digest(did_json.as_bytes());
        
        // 使用私钥生成证明
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        
        // 使用模拟证明（实际应使用真实ZKP）
        let proof = self.zkp_prover.prove_mock(
            &signing_key,
            &did_json,
            nonce,
            hash.as_slice(),
        )?;
        
        log::info!("✅ 证明生成成功");
        Ok(proof)
    }
    
    /// 🔍 验证身份（通过CID + ZKP）
    pub async fn verify_identity_with_zkp(
        &self,
        cid: &str,
        zkp_proof: &[u8],
        nonce: &[u8],
    ) -> Result<IdentityVerification> {
        log::info!("🔍 开始身份验证流程（ZKP版本）");
        log::info!("  CID: {}", cid);
        
        let mut verification_details = Vec::new();
        
        // 步骤1: 从IPFS获取DID文档
        let did_document = get_did_document_from_cid(&self.ipfs_client, cid).await?;
        verification_details.push(format!("✓ DID文档获取成功: {}", did_document.id));
        
        // 步骤2: 计算DID文档哈希
        use blake2::{Blake2s256, Digest};
        let did_json = serde_json::to_string(&did_document)?;
        let hash = Blake2s256::digest(did_json.as_bytes());
        verification_details.push(format!("✓ DID文档哈希计算完成"));
        
        // 步骤3: 提取公钥
        let public_key = self.extract_public_key(&did_document)?;
        verification_details.push(format!("✓ 公钥提取成功"));
        
        // 步骤4: 验证ZKP证明
        let zkp_valid = self.zkp_verifier.verify_mock(
            zkp_proof,
            nonce,
            hash.as_slice(),
            &public_key,
        )?;
        
        if zkp_valid {
            verification_details.push("✓ ZKP验证通过 - DID与CID绑定有效".to_string());
        } else {
            verification_details.push("✗ ZKP验证失败 - DID与CID绑定无效".to_string());
        }
        
        log::info!("✅ 身份验证完成");
        
        Ok(IdentityVerification {
            did: did_document.id.clone(),
            cid: cid.to_string(),
            zkp_verified: zkp_valid,
            verification_details,
            verified_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 🔓 解密PeerID（持有私钥的用户）
    pub fn decrypt_peer_id(
        &self,
        keypair: &KeyPair,
        encrypted: &EncryptedPeerID,
    ) -> Result<PeerId> {
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        decrypt_peer_id_with_secret(&signing_key, encrypted)
    }
    
    /// 从DID文档提取公钥
    fn extract_public_key(&self, did_document: &DIDDocument) -> Result<Vec<u8>> {
        let vm = did_document.verification_method.first()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少验证方法"))?;
        
        // 解码multibase公钥
        let pk_multibase = &vm.public_key_multibase;
        let pk_bs58 = pk_multibase.trim_start_matches('z');
        let public_key = bs58::decode(pk_bs58).into_vec()
            .context("解码公钥失败")?;
        
        Ok(public_key)
    }
    
    /// 从DID文档提取加密的PeerID
    pub fn extract_encrypted_peer_id(&self, did_document: &DIDDocument) -> Result<EncryptedPeerID> {
        let services = did_document.service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少服务端点"))?;
        
        let libp2p_service = services.iter()
            .find(|s| s.service_type == "LibP2PNode")
            .ok_or_else(|| anyhow::anyhow!("未找到LibP2P服务端点"))?;
        
        let endpoint = &libp2p_service.service_endpoint;
        
        let ciphertext_b64 = endpoint.get("encryptedPeerID")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少encryptedPeerID字段"))?;
        
        let nonce_b64 = endpoint.get("nonce")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少nonce字段"))?;
        
        let method = endpoint.get("encryptionMethod")
            .and_then(|v| v.as_str())
            .unwrap_or("AES-256-GCM")
            .to_string();
        
        Ok(EncryptedPeerID {
            ciphertext: general_purpose::STANDARD.decode(ciphertext_b64)
                .context("解码ciphertext失败")?,
            nonce: general_purpose::STANDARD.decode(nonce_b64)
                .context("解码nonce失败")?,
            method,
        })
    }
    
    /// 获取IPFS客户端引用
    pub fn ipfs_client(&self) -> &IpfsClient {
        &self.ipfs_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair as LibP2PKeypair;
    
    #[tokio::test]
    #[ignore] // 需要实际的IPFS服务
    async fn test_register_and_verify_identity() {
        // 创建身份管理器
        let ipfs_client = IpfsClient::new(
            Some("http://localhost:5001".to_string()),
            Some("http://localhost:8080".to_string()),
            None,
            None,
            30,
        );
        
        let manager = IdentityManager::new(ipfs_client);
        
        // 生成密钥对
        let keypair = KeyPair::generate().unwrap();
        let libp2p_keypair = LibP2PKeypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        // 创建智能体信息
        let agent_info = AgentInfo {
            name: "测试智能体".to_string(),
            services: vec![
                ServiceInfo {
                    service_type: "API".to_string(),
                    endpoint: serde_json::json!("https://api.example.com"),
                },
            ],
            description: Some("这是一个测试智能体".to_string()),
            tags: Some(vec!["test".to_string()]),
        };
        
        // 注册身份
        let registration = manager.register_identity(&agent_info, &keypair, &peer_id).await.unwrap();
        println!("✅ 注册成功: {}", registration.did);
        println!("   CID: {}", registration.cid);
        
        // 生成ZKP证明
        let nonce = b"test_nonce_12345";
        let proof = manager.generate_binding_proof(
            &keypair,
            &registration.did_document,
            &registration.cid,
            nonce,
        ).unwrap();
        
        // 验证身份
        let verification = manager.verify_identity_with_zkp(
            &registration.cid,
            &proof.proof,
            nonce,
        ).await.unwrap();
        
        println!("✅ 验证结果: {}", verification.zkp_verified);
        assert!(verification.zkp_verified);
    }
}
