// DIAP Rust SDK - 简化DID文档构建模块
// 使用did:key格式 + ZKP绑定验证（无需IPNS）

use crate::encrypted_peer_id::{encrypt_peer_id, EncryptedPeerID};
use crate::ipfs_client::{IpfsClient, IpfsUploadResult};
use crate::key_manager::KeyPair;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

    /// 服务端点（包含加密的节点ID）
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

    /// PubSub主题列表
    #[serde(rename = "pubsubTopics", skip_serializing_if = "Option::is_none")]
    pub pubsub_topics: Option<Vec<String>>,

    /// 网络监听地址
    #[serde(rename = "networkAddresses", skip_serializing_if = "Option::is_none")]
    pub network_addresses: Option<Vec<String>>,
}

/// DID构建器
pub struct DIDBuilder {
    /// 服务端点列表
    services: Vec<Service>,

    /// PubSub认证主题（可选）
    pubsub_auth_topic: Option<String>,

    /// IPFS客户端
    ipfs_client: IpfsClient,

    /// 可选：Iroh 节点ID（明文字节，发布时会加密写入 DID）
    iroh_node_id: Option<Vec<u8>>,
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

    /// 加密的节点ID
    pub encrypted_peer_id: EncryptedPeerID,

    /// PubSub认证主题
    pub pubsub_auth_topic: String,

    /// IPNS名称（如果已发布到IPNS）
    pub ipns_name: Option<String>,

    /// IPNS值（如果已发布到IPNS）
    pub ipns_value: Option<String>,
}

impl DIDBuilder {
    /// 创建新的DID构建器
    pub fn new(ipfs_client: IpfsClient) -> Self {
        Self {
            services: Vec::new(),
            pubsub_auth_topic: None,
            ipfs_client,
            iroh_node_id: None,
        }
    }

    /// 设置自定义的 PubSub 认证主题
    pub fn set_pubsub_auth_topic<T: Into<String>>(&mut self, topic: T) -> &mut Self {
        self.pubsub_auth_topic = Some(topic.into());
        self
    }

    /// 添加服务端点
    pub fn add_service(&mut self, service_type: &str, endpoint: serde_json::Value) -> &mut Self {
        let service = Service {
            id: format!("#{}", service_type.to_lowercase()),
            service_type: service_type.to_string(),
            service_endpoint: endpoint,
            pubsub_topics: None,
            network_addresses: None,
        };
        self.services.push(service);
        self
    }

    /// 设置 Iroh 节点 ID（明文输入，发布时将被加密进 DID 文档）
    pub fn set_iroh_node_id(&mut self, node_id: &[u8]) -> &mut Self {
        self.iroh_node_id = Some(node_id.to_vec());
        self
    }

    /// 添加PubSub服务端点
    pub fn add_pubsub_service(
        &mut self,
        service_type: &str,
        endpoint: serde_json::Value,
        pubsub_topics: Vec<String>,
        network_addresses: Vec<String>,
    ) -> &mut Self {
        let service = Service {
            id: format!("#{}", service_type.to_lowercase()),
            service_type: service_type.to_string(),
            service_endpoint: endpoint,
            pubsub_topics: Some(pubsub_topics),
            network_addresses: Some(network_addresses),
        };
        self.services.push(service);
        self
    }

    /// 创建并发布包含PubSub信息的DID
    pub async fn create_and_publish_with_pubsub(
        &self,
        keypair: &KeyPair,
        node_id: &str,
        pubsub_topics: Vec<String>,
        network_addresses: Vec<String>,
    ) -> Result<DIDPublishResult> {
        log::info!("🚀 开始DID发布流程（包含PubSub信息）");

        // 步骤1: 加密节点ID
        log::info!("步骤1: 加密节点ID");
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, node_id)?;
        log::info!("✓ 节点ID已加密");

        // 步骤2: 构建包含PubSub信息的DID文档
        log::info!("步骤2: 构建包含PubSub信息的DID文档");
        let did_doc = self.build_did_document_with_pubsub(
            keypair,
            &encrypted_peer_id,
            pubsub_topics,
            network_addresses,
        )?;
        log::info!("✓ DID文档构建完成");
        log::info!("  DID: {}", did_doc.id);

        // 步骤3: 上传到IPFS
        log::info!("步骤3: 上传DID文档到IPFS");
        let upload_result = self.upload_did_document(&did_doc).await?;
        log::info!("✓ 上传完成");
        log::info!("  CID: {}", upload_result.cid);

        log::info!("✅ DID发布成功（包含PubSub信息）");
        log::info!("  DID: {}", keypair.did);
        log::info!("  CID: {}", upload_result.cid);
        log::info!(
            "  PubSub主题: {:?}",
            did_doc
                .service
                .as_ref()
                .and_then(|s| s.first().and_then(|svc| svc.pubsub_topics.as_ref()))
        );
        log::info!(
            "  网络地址: {:?}",
            did_doc
                .service
                .as_ref()
                .and_then(|s| s.first().and_then(|svc| svc.network_addresses.as_ref()))
        );

        let pubsub_topic = self
            .pubsub_auth_topic
            .clone()
            .unwrap_or_else(|| default_pubsub_auth_topic(&keypair.did));

        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            cid: upload_result.cid,
            did_document: did_doc,
            encrypted_peer_id: encrypted_peer_id,
            pubsub_auth_topic: pubsub_topic,
            ipns_name: None,
            ipns_value: None,
        })
    }

    /// 创建并发布DID（简化流程：一次上传）
    pub async fn create_and_publish(
        &self,
        keypair: &KeyPair,
        node_id: &str,
    ) -> Result<DIDPublishResult> {
        log::info!("🚀 开始DID发布流程（简化版）");

        // 步骤1: 加密节点ID
        log::info!("步骤1: 加密节点ID");
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, node_id)?;
        log::info!("✓ 节点ID已加密");

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

        let pubsub_topic = self
            .pubsub_auth_topic
            .clone()
            .unwrap_or_else(|| default_pubsub_auth_topic(&keypair.did));

        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            cid: upload_result.cid,
            did_document: did_doc,
            encrypted_peer_id,
            pubsub_auth_topic: pubsub_topic,
            ipns_name: None,
            ipns_value: None,
        })
    }

    /// 创建并发布DID，自动发布到IPNS
    /// 
    /// # 参数
    /// - `keypair`: 密钥对
    /// - `node_id`: P2P节点ID（字符串格式）
    /// - `ipns_key_name`: IPNS key 名称（如果为 None，则不发布到IPNS）
    /// - `use_direct_publish`: 是否使用直接发布（allow-offline=false），确保DHT传播
    /// - `ipns_lifetime`: IPNS记录生命周期（默认 "8760h"，即1年）
    /// - `ipns_ttl`: IPNS缓存时间（默认 "1h"）
    pub async fn create_and_publish_with_ipns(
        &self,
        keypair: &KeyPair,
        node_id: &str,
        ipns_key_name: Option<&str>,
        use_direct_publish: bool,
        ipns_lifetime: Option<&str>,
        ipns_ttl: Option<&str>,
    ) -> Result<DIDPublishResult> {
        log::info!("🚀 开始DID发布流程（包含IPNS自动发布）");

        // 步骤1: 加密节点ID
        log::info!("步骤1: 加密节点ID");
        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, node_id)?;
        log::info!("✓ 节点ID已加密");

        // 步骤2: 构建DID文档
        log::info!("步骤2: 构建DID文档");
        let did_doc = self.build_did_document(keypair, &encrypted_peer_id)?;
        log::info!("✓ DID文档构建完成");
        log::info!("  DID: {}", did_doc.id);

        // 步骤3: 上传到IPFS
        log::info!("步骤3: 上传DID文档到IPFS");
        let upload_result = self.upload_did_document(&did_doc).await?;
        log::info!("✓ 上传完成");
        log::info!("  CID: {}", upload_result.cid);

        // 步骤4: 可选发布到IPNS（带超时保护）
        let (ipns_name, ipns_value) = if let Some(key_name) = ipns_key_name {
            log::info!("步骤4: 发布到IPNS（key={}, direct={}）", key_name, use_direct_publish);
            log::info!("   开始IPNS发布流程...");
            
            let lifetime = ipns_lifetime.unwrap_or("8760h");
            let ttl = ipns_ttl.unwrap_or("1h");
            
            // 使用超时包装，避免IPNS发布阻塞太久（最多等待30秒）
            use tokio::time::{timeout, Duration};
            let ipns_timeout = Duration::from_secs(30);
            
            let ipns_result = if use_direct_publish {
                // 使用直接发布，确保DHT传播
                match timeout(ipns_timeout, self.ipfs_client.publish_after_upload_direct(
                    &upload_result.cid,
                    key_name,
                    lifetime,
                    ttl,
                )).await {
                    Ok(Ok(ipns)) => {
                        log::info!("✅ IPNS记录已发布到DHT: /ipns/{}", ipns.name);
                        Some((ipns.name, ipns.value))
                    }
                    Ok(Err(e)) => {
                        log::warn!("⚠️ IPNS直接发布失败（不影响主流程）: {}", e);
                        log::warn!("   提示: 节点可能未连接到DHT网络，将尝试快速发布");
                        
                        // 回退到快速发布
                        match timeout(ipns_timeout, self.ipfs_client.publish_after_upload(
                            &upload_result.cid,
                            key_name,
                            lifetime,
                            ttl,
                        )).await {
                            Ok(Ok(ipns)) => {
                                log::info!("✅ IPNS记录已发布（快速模式）: /ipns/{}", ipns.name);
                                Some((ipns.name, ipns.value))
                            }
                            Ok(Err(e2)) => {
                                log::warn!("⚠️ IPNS发布失败（不影响主流程）: {}", e2);
                                None
                            }
                            Err(_) => {
                                log::warn!("⚠️ IPNS发布超时（30秒），跳过IPNS发布");
                                None
                            }
                        }
                    }
                    Err(_) => {
                        log::warn!("⚠️ IPNS直接发布超时（30秒），尝试快速发布");
                        // 回退到快速发布
                        match timeout(ipns_timeout, self.ipfs_client.publish_after_upload(
                            &upload_result.cid,
                            key_name,
                            lifetime,
                            ttl,
                        )).await {
                            Ok(Ok(ipns)) => {
                                log::info!("✅ IPNS记录已发布（快速模式）: /ipns/{}", ipns.name);
                                Some((ipns.name, ipns.value))
                            }
                            Ok(Err(e2)) => {
                                log::warn!("⚠️ IPNS发布失败（不影响主流程）: {}", e2);
                                None
                            }
                            Err(_) => {
                                log::warn!("⚠️ IPNS发布超时（30秒），跳过IPNS发布");
                                None
                            }
                        }
                    }
                }
            } else {
                // 使用快速发布
                match timeout(ipns_timeout, self.ipfs_client.publish_after_upload(
                    &upload_result.cid,
                    key_name,
                    lifetime,
                    ttl,
                )).await {
                    Ok(Ok(ipns)) => {
                        log::info!("✅ IPNS记录已发布: /ipns/{}", ipns.name);
                        Some((ipns.name, ipns.value))
                    }
                    Ok(Err(e)) => {
                        log::warn!("⚠️ IPNS发布失败（不影响主流程）: {}", e);
                        None
                    }
                    Err(_) => {
                        log::warn!("⚠️ IPNS发布超时（30秒），跳过IPNS发布");
                        None
                    }
                }
            };
            
            if let Some((ref name, ref value)) = ipns_result {
                log::info!("  IPNS: /ipns/{} -> {}", name, value);
            }
            
            ipns_result.map(|(n, v)| (Some(n), Some(v))).unwrap_or((None, None))
        } else {
            (None, None)
        };

        log::info!("✅ DID发布成功");
        log::info!("  DID: {}", keypair.did);
        log::info!("  CID: {}", upload_result.cid);
        if ipns_name.is_some() {
            log::info!("  IPNS: /ipns/{}", ipns_name.as_ref().unwrap());
        }
        log::info!("  绑定关系: 通过ZKP验证");

        let pubsub_topic = self
            .pubsub_auth_topic
            .clone()
            .unwrap_or_else(|| default_pubsub_auth_topic(&keypair.did));

        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            cid: upload_result.cid,
            did_document: did_doc,
            encrypted_peer_id,
            pubsub_auth_topic: pubsub_topic,
            ipns_name,
            ipns_value,
        })
    }

    /// 构建DID文档
    fn build_did_document(
        &self,
        keypair: &KeyPair,
        encrypted_peer_id: &EncryptedPeerID,
    ) -> Result<DIDDocument> {
        // 编码公钥为multibase格式（包含 Ed25519 multicodec 前缀 0xed 0x01）
        let mut multicodec_pubkey = vec![0xed, 0x01];
        multicodec_pubkey.extend_from_slice(&keypair.public_key);
        let public_key_multibase = format!("z{}", bs58::encode(&multicodec_pubkey).into_string());

        // 创建验证方法
        let verification_method = VerificationMethod {
            id: format!("{}#key-1", keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: keypair.did.clone(),
            public_key_multibase,
        };

        // 添加加密的PeerID服务（隐私保护 - AES-256-GCM）
        let mut services = self.services.clone();

        // 确保 pubsub-auth 服务存在
        let pubsub_auth_topic = self
            .pubsub_auth_topic
            .clone()
            .unwrap_or_else(|| default_pubsub_auth_topic(&keypair.did));
        insert_or_replace_pubsub_auth_service(&mut services, &keypair.did, &pubsub_auth_topic);

        let libp2p_service = Service {
            id: "#libp2p".to_string(),
            service_type: "LibP2PNode".to_string(),
            service_endpoint: serde_json::json!({
                "ciphertext": general_purpose::STANDARD.encode(&encrypted_peer_id.ciphertext),
                "nonce": general_purpose::STANDARD.encode(&encrypted_peer_id.nonce),
                "signature": general_purpose::STANDARD.encode(&encrypted_peer_id.signature),
                "method": encrypted_peer_id.method,
            }),
            pubsub_topics: None,
            network_addresses: None,
        };
        services.insert(0, libp2p_service);

        // 可选：添加 IrohNode 服务（加密 Iroh ID）
        if let Some(ref iroh_plain) = self.iroh_node_id {
            use crate::encrypted_iroh_id::encrypt_iroh_id;
            let signing_key = SigningKey::from_bytes(&keypair.private_key);
            let enc = encrypt_iroh_id(&signing_key, iroh_plain)?;
            let iroh_service = Service {
                id: "#iroh".to_string(),
                service_type: "IrohNode".to_string(),
                service_endpoint: serde_json::json!({
                    "ciphertext": base64::engine::general_purpose::STANDARD.encode(&enc.ciphertext),
                    "nonce": base64::engine::general_purpose::STANDARD.encode(&enc.nonce),
                    "signature": base64::engine::general_purpose::STANDARD.encode(&enc.signature),
                    "method": enc.method,
                    "protocol": "iroh",
                    "version": "1.0.0"
                }),
                pubsub_topics: None,
                network_addresses: None,
            };
            services.insert(1, iroh_service);
        }

        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: keypair.did.clone(),
            verification_method: vec![verification_method],
            authentication: vec![format!("{}#key-1", keypair.did)],
            service: if services.is_empty() {
                None
            } else {
                Some(services)
            },
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 构建包含PubSub信息的DID文档
    fn build_did_document_with_pubsub(
        &self,
        keypair: &KeyPair,
        encrypted_peer_id: &EncryptedPeerID,
        pubsub_topics: Vec<String>,
        network_addresses: Vec<String>,
    ) -> Result<DIDDocument> {
        // 构建验证方法
        // 编码公钥为multibase格式（包含 Ed25519 multicodec 前缀 0xed 0x01）
        let mut multicodec_pubkey = vec![0xed, 0x01];
        multicodec_pubkey.extend_from_slice(&keypair.public_key);
        let public_key_multibase = format!("z{}", bs58::encode(&multicodec_pubkey).into_string());

        let verification_method = VerificationMethod {
            id: format!("{}#key-1", keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: keypair.did.clone(),
            public_key_multibase,
        };

        // 构建服务列表
        let mut services = self.services.clone();

        // 添加libp2p服务（包含PubSub信息）
        let libp2p_service = Service {
            id: format!("{}#libp2p", keypair.did),
            service_type: "libp2p".to_string(),
            service_endpoint: serde_json::json!({
                "ciphertext": general_purpose::STANDARD.encode(&encrypted_peer_id.ciphertext),
                "nonce": general_purpose::STANDARD.encode(&encrypted_peer_id.nonce),
                "signature": general_purpose::STANDARD.encode(&encrypted_peer_id.signature),
                "method": encrypted_peer_id.method,
                "protocol": "libp2p",
                "version": "1.0.0"
            }),
            pubsub_topics: Some(pubsub_topics),
            network_addresses: Some(network_addresses),
        };
        services.insert(0, libp2p_service);

        // 可选：添加 IrohNode 服务（加密 Iroh ID）
        if let Some(ref iroh_plain) = self.iroh_node_id {
            use crate::encrypted_iroh_id::encrypt_iroh_id;
            let signing_key = SigningKey::from_bytes(&keypair.private_key);
            let enc = encrypt_iroh_id(&signing_key, iroh_plain)?;
            let iroh_service = Service {
                id: "#iroh".to_string(),
                service_type: "IrohNode".to_string(),
                service_endpoint: serde_json::json!({
                    "ciphertext": base64::engine::general_purpose::STANDARD.encode(&enc.ciphertext),
                    "nonce": base64::engine::general_purpose::STANDARD.encode(&enc.nonce),
                    "signature": base64::engine::general_purpose::STANDARD.encode(&enc.signature),
                    "method": enc.method,
                    "protocol": "iroh",
                    "version": "1.0.0"
                }),
                    pubsub_topics: None,
                    network_addresses: None,
            };
            services.insert(1, iroh_service);
        }

        // 确保 pubsub-auth 服务存在
        let pubsub_auth_topic = self
            .pubsub_auth_topic
            .clone()
            .unwrap_or_else(|| default_pubsub_auth_topic(&keypair.did));
        insert_or_replace_pubsub_auth_service(&mut services, &keypair.did, &pubsub_auth_topic);

        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: keypair.did.clone(),
            verification_method: vec![verification_method],
            authentication: vec![format!("{}#key-1", keypair.did)],
            service: if services.is_empty() {
                None
            } else {
                Some(services)
            },
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 上传DID文档到IPFS
    async fn upload_did_document(&self, did_doc: &DIDDocument) -> Result<IpfsUploadResult> {
        let json = serde_json::to_string_pretty(did_doc).context("序列化DID文档失败")?;

        self.ipfs_client
            .upload(&json, "did.json")
            .await
            .context("上传DID文档到IPFS失败")
    }
}

fn default_pubsub_auth_topic(did: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(did.as_bytes());
    let hash = hasher.finalize();
    let short = &hash[..8];
    format!("diap-auth-{}", hex::encode(short))
}

fn insert_or_replace_pubsub_auth_service(services: &mut Vec<Service>, did: &str, topic: &str) {
    let endpoint = serde_json::json!({
        "topic": topic,
        "protocol": "pubsub",
    });
    let service = Service {
        id: format!("{}#pubsub-auth", did),
        service_type: "PubSubAuth".to_string(),
        service_endpoint: endpoint,
        pubsub_topics: None,
        network_addresses: None,
    };

    if let Some(pos) = services
        .iter()
        .position(|s| s.service_type.eq_ignore_ascii_case("PubSubAuth"))
    {
        services[pos] = service;
    } else {
        services.insert(0, service);
    }
}

/// 从IPFS CID获取DID文档
pub async fn get_did_document_from_cid(ipfs_client: &IpfsClient, cid: &str) -> Result<DIDDocument> {
    log::info!("从IPFS获取DID文档: {}", cid);

    let content = ipfs_client
        .get(cid)
        .await
        .context("从IPFS获取DID文档失败")?;

    let did_doc: DIDDocument = serde_json::from_str(&content).context("解析DID文档失败")?;

    log::info!("✓ DID文档获取成功: {}", did_doc.id);

    Ok(did_doc)
}

/// 验证DID文档的完整性（改进版：支持多种哈希算法）
/// 验证DID文档的哈希是否与CID的multihash部分匹配
pub fn verify_did_document_integrity(did_doc: &DIDDocument, expected_cid: &str) -> Result<bool> {
    use blake2::{Blake2b512, Blake2s256};
    use cid::Cid;
    use sha2::{Digest, Sha256, Sha512};
    use std::str::FromStr;

    log::info!("验证DID文档完整性与CID绑定（支持多种哈希算法）");

    // 1. 序列化DID文档（使用确定性序列化）
    let json = serde_json::to_string(did_doc).context("序列化DID文档失败")?;

    log::debug!("  DID文档大小: {} 字节", json.len());

    // 2. 解析CID
    let cid = Cid::from_str(expected_cid).context("解析CID失败")?;

    log::debug!("  CID版本: {:?}", cid.version());
    log::debug!("  CID codec: {:?}", cid.codec());

    // 3. 提取CID的multihash部分
    let multihash = cid.hash();
    let hash_code = multihash.code();
    let hash_digest = multihash.digest();

    log::debug!("  Multihash code: 0x{:x}", hash_code);
    log::debug!("  Multihash digest: {}", hex::encode(hash_digest));

    // 4. 根据哈希算法计算文档哈希
    let computed_hash: Vec<u8> = match hash_code {
        0x12 => {
            // SHA-256
            log::debug!("  使用SHA-256计算哈希");
            Sha256::digest(json.as_bytes()).to_vec()
        }
        0x13 => {
            // SHA-512
            log::debug!("  使用SHA-512计算哈希");
            Sha512::digest(json.as_bytes()).to_vec()
        }
        0xb220 => {
            // Blake2b-512
            log::debug!("  使用Blake2b-512计算哈希");
            Blake2b512::digest(json.as_bytes()).to_vec()
        }
        0xb260 => {
            // Blake2s-256
            log::debug!("  使用Blake2s-256计算哈希");
            Blake2s256::digest(json.as_bytes()).to_vec()
        }
        _ => {
            log::warn!("  ⚠️ 不支持的哈希算法: 0x{:x}", hash_code);
            // 默认使用SHA-256
            log::debug!("  回退到SHA-256");
            Sha256::digest(json.as_bytes()).to_vec()
        }
    };

    log::debug!("  计算的哈希: {}", hex::encode(&computed_hash));

    // 5. 比较哈希值
    let hashes_match = computed_hash.as_slice() == hash_digest;

    if hashes_match {
        log::info!("✅ DID文档哈希与CID匹配");
    } else {
        log::warn!("❌ DID文档哈希与CID不匹配");
        log::debug!("  预期: {}", hex::encode(hash_digest));
        log::debug!("  实际: {}", hex::encode(&computed_hash));
        log::debug!("  哈希算法: 0x{:x}", hash_code);
    }

    Ok(hashes_match)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_did_document() {
        let keypair = KeyPair::generate().unwrap();
        let node_id = "12D3KooWExamplePeerIdForTesting"; // 使用示例节点ID

        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let builder = DIDBuilder::new(ipfs_client);

        let signing_key = SigningKey::from_bytes(&keypair.private_key);
        let encrypted_peer_id = encrypt_peer_id(&signing_key, node_id).unwrap();

        let did_doc = builder
            .build_did_document(&keypair, &encrypted_peer_id)
            .unwrap();

        assert_eq!(did_doc.id, keypair.did);
        assert_eq!(did_doc.verification_method.len(), 1);
        assert!(did_doc.service.is_some());

        println!("✓ DID文档构建测试通过");
        println!("  DID: {}", did_doc.id);
    }
}
