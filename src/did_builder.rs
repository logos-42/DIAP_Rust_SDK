// DIAP Rust SDK - 简化DID文档构建模块
// 使用did:key格式 + ZKP绑定验证（无需IPNS）

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::key_manager::KeyPair;
use crate::ipfs_client::{IpfsClient, IpfsUploadResult};
use crate::encrypted_peer_id::{EncryptedPeerID, encrypt_peer_id};
use libp2p::PeerId;
use ed25519_dalek::SigningKey;
use base64::{Engine as _, engine::general_purpose};

/// DID文档（简化版，使用did:key）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    /// DID标识符（did:key格式）
    pub id: String,
    
    /// 验证方法
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    
    /// 认证方法
    pub authentication: Vec<String>,
    
    /// 服务端点（包含加密的PeerID）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<Service>>,
    
    /// 创建时间
    pub created: String,
}

/// 验证方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    
    #[serde(rename = "type")]
    pub vm_type: String,
    
    pub controller: String,
    
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// 服务端点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    
    #[serde(rename = "type")]
    pub service_type: String,
    
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: serde_json::Value,
}

/// DID构建器
pub struct DIDBuilder {
    /// 服务端点列表
    services: Vec<Service>,
    
    /// IPFS客户端
    ipfs_client: IpfsClient,
}

/// DID发布结果
#[derive(Debug, Clone)]
pub struct DIDPublishResult {
    /// DID标识符（did:key格式）
    pub did: String,
    
    /// IPFS CID（DID文档的内容地址）
    pub cid: String,
    
    /// DID文档
    pub did_document: DIDDocument,
    
    /// 加密的PeerID
    pub encrypted_peer_id: EncryptedPeerID,
}

impl DIDBuilder {
    /// 创建新的DID构建器
    pub fn new(ipfs_client: IpfsClient) -> Self {
        Self {
            services: Vec::new(),
            ipfs_client,
        }
    }
    
    /// 添加服务端点
    pub fn add_service(&mut self, service_type: &str, endpoint: serde_json::Value) -> &mut Self {
        let service = Service {
            id: format!("#{}", service_type.to_lowercase()),
            service_type: service_type.to_string(),
            service_endpoint: endpoint,
        };
        self.services.push(service);
        self
    }
    
    /// 创建并发布DID（简化流程：一次上传）
    pub async fn create_and_publish(
        &self,
        keypair: &KeyPair,
        libp2p_peer_id: &PeerId,
    ) -> Result<DIDPublishResult> {
        log::info!("🚀 开始DID发布流程（简化版）");
        
        // 步骤1: 加密PeerID
        log::info!("步骤1: 加密libp2p PeerID");
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, libp2p_peer_id)?;
        log::info!("✓ PeerID已加密");
        
        // 步骤2: 构建DID文档
        log::info!("步骤2: 构建DID文档");
        let did_doc = self.build_did_document(keypair, &encrypted_peer_id)?;
        log::info!("✓ DID文档构建完成");
        log::info!("  DID: {}", did_doc.id);
        
        // 步骤3: 上传到IPFS（仅一次）
        log::info!("步骤3: 上传DID文档到IPFS");
        let upload_result = self.upload_did_document(&did_doc).await?;
        log::info!("✓ 上传完成");
        log::info!("  CID: {}", upload_result.cid);
        
        log::info!("✅ DID发布成功");
        log::info!("  DID: {}", keypair.did);
        log::info!("  CID: {}", upload_result.cid);
        log::info!("  绑定关系: 通过ZKP验证");
        
        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            cid: upload_result.cid,
            did_document: did_doc,
            encrypted_peer_id,
        })
    }
    
    /// 构建DID文档
    fn build_did_document(
        &self,
        keypair: &KeyPair,
        encrypted_peer_id: &EncryptedPeerID,
    ) -> Result<DIDDocument> {
        // 编码公钥为multibase格式
        let public_key_multibase = format!("z{}", bs58::encode(&keypair.public_key).into_string());
        
        // 创建验证方法
        let verification_method = VerificationMethod {
            id: format!("{}#key-1", keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: keypair.did.clone(),
            public_key_multibase,
        };
        
        // 添加加密的PeerID服务
        let mut services = self.services.clone();
        let libp2p_service = Service {
            id: "#libp2p".to_string(),
            service_type: "LibP2PNode".to_string(),
            service_endpoint: serde_json::json!({
                "encryptedPeerID": general_purpose::STANDARD.encode(&encrypted_peer_id.ciphertext),
                "nonce": general_purpose::STANDARD.encode(&encrypted_peer_id.nonce),
                "encryptionMethod": encrypted_peer_id.method,
            }),
        };
        services.insert(0, libp2p_service);
        
        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: keypair.did.clone(),
            verification_method: vec![verification_method],
            authentication: vec![format!("{}#key-1", keypair.did)],
            service: if services.is_empty() { None } else { Some(services) },
            created: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 上传DID文档到IPFS
    async fn upload_did_document(&self, did_doc: &DIDDocument) -> Result<IpfsUploadResult> {
        let json = serde_json::to_string_pretty(did_doc)
            .context("序列化DID文档失败")?;
        
        self.ipfs_client
            .upload(&json, "did.json")
            .await
            .context("上传DID文档到IPFS失败")
    }
}

/// 从IPFS CID获取DID文档
pub async fn get_did_document_from_cid(
    ipfs_client: &IpfsClient,
    cid: &str,
) -> Result<DIDDocument> {
    log::info!("从IPFS获取DID文档: {}", cid);
    
    let content = ipfs_client.get(cid).await
        .context("从IPFS获取DID文档失败")?;
    
    let did_doc: DIDDocument = serde_json::from_str(&content)
        .context("解析DID文档失败")?;
    
    log::info!("✓ DID文档获取成功: {}", did_doc.id);
    
    Ok(did_doc)
}

/// 验证DID文档的完整性（通过哈希）
/// 验证DID文档的SHA-256哈希是否与CID的multihash部分匹配
pub fn verify_did_document_integrity(
    did_doc: &DIDDocument,
    expected_cid: &str,
) -> Result<bool> {
    use sha2::{Sha256, Digest};
    use cid::Cid;
    use std::str::FromStr;
    
    log::info!("验证DID文档完整性与CID绑定");
    
    // 1. 序列化DID文档（使用确定性序列化）
    let json = serde_json::to_string(did_doc)
        .context("序列化DID文档失败")?;
    
    log::debug!("  DID文档大小: {} 字节", json.len());
    
    // 2. 计算文档的SHA-256哈希
    let computed_hash = Sha256::digest(json.as_bytes());
    log::debug!("  计算的哈希: {}", hex::encode(&computed_hash));
    
    // 3. 解析CID
    let cid = Cid::from_str(expected_cid)
        .context("解析CID失败")?;
    
    log::debug!("  CID版本: {:?}", cid.version());
    log::debug!("  CID codec: {:?}", cid.codec());
    
    // 4. 提取CID的multihash部分
    let multihash = cid.hash();
    let hash_code = multihash.code();
    let hash_digest = multihash.digest();
    
    log::debug!("  Multihash code: 0x{:x}", hash_code);
    log::debug!("  Multihash digest: {}", hex::encode(hash_digest));
    
    // 5. 验证哈希算法（应该是SHA-256, code = 0x12）
    if hash_code != 0x12 {
        log::warn!("  ⚠️ CID使用的不是SHA-256哈希（code: 0x{:x}）", hash_code);
        // 注意：IPFS可能使用不同的哈希算法，这是正常的
        // 我们仍然可以验证，但需要相应地计算哈希
    }
    
    // 6. 比较哈希值
    let hashes_match = computed_hash.as_slice() == hash_digest;
    
    if hashes_match {
        log::info!("✅ DID文档哈希与CID匹配");
    } else {
        log::warn!("❌ DID文档哈希与CID不匹配");
        log::debug!("  预期: {}", hex::encode(hash_digest));
        log::debug!("  实际: {}", hex::encode(&computed_hash));
    }
    
    Ok(hashes_match)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair as LibP2PKeypair;
    
    #[test]
    fn test_build_did_document() {
        let keypair = KeyPair::generate().unwrap();
        let libp2p_keypair = LibP2PKeypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());
        
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let builder = DIDBuilder::new(ipfs_client);
        
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, &peer_id).unwrap();
        
        let did_doc = builder.build_did_document(&keypair, &encrypted_peer_id).unwrap();
        
        assert_eq!(did_doc.id, keypair.did);
        assert_eq!(did_doc.verification_method.len(), 1);
        assert!(did_doc.service.is_some());
        
        println!("✓ DID文档构建测试通过");
        println!("  DID: {}", did_doc.id);
    }
}
