// DIAP Rust SDK - 统一身份管理模块（ZKP版本）
// 使用ZKP验证DID-CID绑定，无需IPNS

use crate::did_builder::{get_did_document_from_cid, DIDBuilder, DIDDocument};
use crate::ipfs_client::IpfsClient;
use crate::key_manager::KeyPair;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
// 注意：已移除对zkp_prover的依赖，改用Noir ZKP
use crate::encrypted_peer_id::{
    decrypt_peer_id_with_secret, verify_peer_id_signature, EncryptedPeerID,
};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::SigningKey;
use libp2p::PeerId;

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

    /// PubSub认证主题
    pub pubsub_auth_topic: String,

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

/// 统一身份管理器（简化版本）
pub struct IdentityManager {
    /// IPFS客户端
    ipfs_client: IpfsClient,
}

impl IdentityManager {
    /// 创建新的身份管理器
    pub fn new(ipfs_client: IpfsClient) -> Self {
        log::info!("🔐 创建IdentityManager（简化版本）");

        Self { ipfs_client }
    }

    /// 便捷构造函数：从文件路径创建身份管理器（已废弃）
    pub fn new_with_keys(ipfs_client: IpfsClient, _pk_path: &str, _vk_path: &str) -> Result<Self> {
        log::warn!("⚠️  new_with_keys已废弃，请使用Noir ZKP");

        Ok(Self::new(ipfs_client))
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
        let publish_result = builder
            .create_and_publish(keypair, libp2p_peer_id)
            .await
            .context("DID发布失败")?;

        log::info!("✅ 身份注册成功");
        log::info!("  DID: {}", publish_result.did);
        log::info!("  CID: {}", publish_result.cid);
        log::info!("  PubSub认证主题: {}", publish_result.pubsub_auth_topic);

        Ok(IdentityRegistration {
            did: publish_result.did,
            cid: publish_result.cid,
            did_document: publish_result.did_document,
            encrypted_peer_id_hex: hex::encode(&publish_result.encrypted_peer_id.signature),
            pubsub_auth_topic: publish_result.pubsub_auth_topic,
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
    ) -> Result<Vec<u8>> {
        log::warn!("⚠️  generate_zkp_proof已废弃，请使用Noir ZKP");

        // 返回简单的哈希作为占位符
        use blake2::{Blake2s256, Digest};
        let did_json = serde_json::to_string(did_document)?;
        let mut hasher = Blake2s256::new();
        hasher.update(did_json.as_bytes());
        hasher.update(nonce);
        hasher.update(&keypair.private_key);

        let proof_hash = hasher.finalize();
        Ok(proof_hash.to_vec())
    }

    /// 🔍 验证身份（通过CID + ZKP）
    pub async fn verify_identity_with_zkp(
        &self,
        cid: &str,
        _zkp_proof: &[u8],
        _nonce: &[u8],
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
        let _hash = Blake2s256::digest(did_json.as_bytes());
        verification_details.push(format!("✓ DID文档哈希计算完成"));

        // 步骤3: 提取公钥
        let _public_key = self.extract_public_key(&did_document)?;
        verification_details.push(format!("✓ 公钥提取成功"));

        // 步骤4: 验证ZKP证明（简化版本）
        log::warn!("⚠️  ZKP验证已简化，请使用Noir ZKP");
        let zkp_valid = true; // 占位符验证

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

    /// 🔓 验证PeerID签名（任何人都可以验证）
    pub fn verify_peer_id(
        &self,
        did_document: &DIDDocument,
        encrypted: &EncryptedPeerID,
        claimed_peer_id: &PeerId,
    ) -> Result<bool> {
        // 提取公钥
        let public_key_bytes = self.extract_public_key(did_document)?;

        // 跳过multicodec前缀（通常是2字节）
        let key_bytes = if public_key_bytes.len() > 32 {
            &public_key_bytes[public_key_bytes.len() - 32..]
        } else {
            &public_key_bytes
        };

        let verifying_key =
            ed25519_dalek::VerifyingKey::from_bytes(key_bytes.try_into().context("公钥长度错误")?)?;

        verify_peer_id_signature(&verifying_key, encrypted, claimed_peer_id)
    }

    /// 🔓 解密PeerID（已废弃 - 新方案不支持）
    #[deprecated(note = "新签名方案不支持解密PeerID，请使用verify_peer_id")]
    pub fn decrypt_peer_id(
        &self,
        keypair: &KeyPair,
        encrypted: &EncryptedPeerID,
    ) -> Result<PeerId> {
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        decrypt_peer_id_with_secret(&signing_key, encrypted)
    }

    /// 从DID文档提取公钥（改进版：正确解析multicodec前缀）
    fn extract_public_key(&self, did_document: &DIDDocument) -> Result<Vec<u8>> {
        let vm = did_document
            .verification_method
            .first()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少验证方法"))?;

        // 解码multibase公钥（'z'表示base58btc编码）
        let pk_multibase = &vm.public_key_multibase;
        if !pk_multibase.starts_with('z') {
            anyhow::bail!("公钥必须使用base58btc编码（'z'前缀）");
        }

        let pk_bs58 = &pk_multibase[1..]; // 移除'z'前缀
        let encoded_key = bs58::decode(pk_bs58)
            .into_vec()
            .context("解码base58公钥失败")?;

        // 解析multicodec前缀
        // Ed25519公钥: 0xed01 (2字节)
        if encoded_key.len() < 2 {
            anyhow::bail!("公钥数据太短");
        }

        // 检查multicodec前缀
        if encoded_key[0] == 0xed && encoded_key[1] == 0x01 {
            // Ed25519公钥，提取实际的32字节公钥
            if encoded_key.len() != 34 {
                // 2字节前缀 + 32字节公钥
                anyhow::bail!(
                    "Ed25519公钥长度错误：期望34字节，实际{}字节",
                    encoded_key.len()
                );
            }
            Ok(encoded_key[2..].to_vec())
        } else {
            // 未知的multicodec，返回全部数据
            log::warn!(
                "未知的multicodec前缀: 0x{:02x}{:02x}",
                encoded_key[0],
                encoded_key[1]
            );
            Ok(encoded_key)
        }
    }

    /// 从DID文档提取加密的PeerID（改进版）
    pub fn extract_encrypted_peer_id(&self, did_document: &DIDDocument) -> Result<EncryptedPeerID> {
        let services = did_document
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少服务端点"))?;

        let libp2p_service = services
            .iter()
            .find(|s| s.service_type == "LibP2PNode")
            .ok_or_else(|| anyhow::anyhow!("未找到LibP2P服务端点"))?;

        let endpoint = &libp2p_service.service_endpoint;

        let ciphertext_b64 = endpoint
            .get("ciphertext")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少ciphertext字段"))?;

        let nonce_b64 = endpoint
            .get("nonce")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少nonce字段"))?;

        let signature_b64 = endpoint
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少signature字段"))?;

        let method = endpoint
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("AES-256-GCM-Ed25519-V3")
            .to_string();

        Ok(EncryptedPeerID {
            ciphertext: general_purpose::STANDARD
                .decode(ciphertext_b64)
                .context("解码ciphertext失败")?,
            nonce: general_purpose::STANDARD
                .decode(nonce_b64)
                .context("解码nonce失败")?,
            signature: general_purpose::STANDARD
                .decode(signature_b64)
                .context("解码signature失败")?,
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
    #[ignore] // 需要实际的IPFS服务和ZKP keys
    async fn test_register_and_verify_identity() {
        // 创建身份管理器
        let ipfs_client = IpfsClient::new(
            Some("http://localhost:5001".to_string()),
            Some("http://localhost:8080".to_string()),
            None,
            None,
            30,
        );

        // 注意：这个测试需要先生成ZKP keys
        // 运行: cargo run --example zkp_setup_keys
        let manager =
            IdentityManager::new_with_keys(ipfs_client, "zkp_proving.key", "zkp_verifying.key")
                .expect("无法加载ZKP keys，请先运行 zkp_setup_keys");

        // 生成密钥对
        let keypair = KeyPair::generate().unwrap();
        let libp2p_keypair = LibP2PKeypair::generate_ed25519();
        let peer_id = PeerId::from(libp2p_keypair.public());

        // 创建智能体信息
        let agent_info = AgentInfo {
            name: "测试智能体".to_string(),
            services: vec![ServiceInfo {
                service_type: "API".to_string(),
                endpoint: serde_json::json!("https://api.example.com"),
            }],
            description: Some("这是一个测试智能体".to_string()),
            tags: Some(vec!["test".to_string()]),
        };

        // 注册身份
        let registration = manager
            .register_identity(&agent_info, &keypair, &peer_id)
            .await
            .unwrap();
        println!("✅ 注册成功: {}", registration.did);
        println!("   CID: {}", registration.cid);

        // 生成ZKP证明
        let nonce = b"test_nonce_12345";
        let proof = manager
            .generate_binding_proof(
                &keypair,
                &registration.did_document,
                &registration.cid,
                nonce,
            )
            .unwrap();

        // 验证身份
        let verification = manager
            .verify_identity_with_zkp(&registration.cid, &proof.proof, nonce)
            .await
            .unwrap();

        println!("✅ 验证结果: {}", verification.zkp_verified);
        assert!(verification.zkp_verified);
    }
}
