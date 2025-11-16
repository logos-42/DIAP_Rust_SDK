// DIAP Rust SDK - IPFS Pubsub认证通讯模块
// 基于libp2p gossipsub实现认证的发布/订阅通信

use anyhow::{Context, Result};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::did_cache::DIDCache;
use crate::identity_manager::IdentityManager;
use crate::key_manager::KeyPair;
use crate::nonce_manager::NonceManager;

/// PubSub消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PubSubMessageType {
    /// 身份验证请求
    AuthRequest,
    /// 身份验证响应
    AuthResponse,
    /// 资源访问请求
    ResourceRequest,
    /// 资源访问响应
    ResourceResponse,
    /// 心跳消息
    Heartbeat,
    /// 自定义消息
    Custom(String),
}

/// 认证的Pubsub消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedMessage {
    /// 消息ID
    pub message_id: String,

    /// 消息类型
    pub message_type: PubSubMessageType,

    /// 发送者DID
    pub from_did: String,

    /// 接收者DID（可选，为空表示广播）
    pub to_did: Option<String>,

    /// 发送者PeerID
    pub from_peer_id: String,

    /// DID文档的CID
    pub did_cid: String,

    /// 主题
    pub topic: String,

    /// 消息内容（原始数据）
    pub content: Vec<u8>,

    /// Nonce（防重放）
    pub nonce: String,

    /// ZKP证明
    pub zkp_proof: Vec<u8>,

    /// 内容签名（使用DID私钥）
    pub signature: Vec<u8>,

    /// 时间戳
    pub timestamp: u64,
}

/// PubSub 认证请求负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubsubAuthRequestPayload {
    /// 目标身份的 CID
    pub target_cid: String,
    /// 建议的响应主题（可选）
    pub response_topic: Option<String>,
    /// 附加说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// PubSub 认证响应负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubsubAuthResponsePayload {
    /// 请求方的 nonce（用于匹配请求）
    pub request_nonce: String,
    /// 目标身份的 CID
    pub target_cid: String,
    /// 是否成功生成证明
    pub success: bool,
    /// 附加说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Pubsub消息验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageVerification {
    /// 是否验证通过
    pub verified: bool,

    /// 发送者DID
    pub from_did: String,

    /// 验证详情
    pub details: Vec<String>,

    /// 验证时间戳
    pub verified_at: u64,
}

/// 主题授权策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopicPolicy {
    /// 允许所有经过认证的用户
    AllowAuthenticated,

    /// 仅允许特定DID列表
    AllowList(Vec<String>),

    /// 拒绝特定DID列表
    DenyList(Vec<String>),

    /// 自定义验证函数
    Custom,
}

/// 主题配置
#[derive(Debug, Clone)]
pub struct TopicConfig {
    /// 主题名称
    pub name: String,

    /// 授权策略
    pub policy: TopicPolicy,

    /// 是否需要ZKP验证
    pub require_zkp: bool,

    /// 是否需要签名验证
    pub require_signature: bool,
}

/// Pubsub认证器
pub struct PubsubAuthenticator {
    /// 身份管理器
    identity_manager: Arc<IdentityManager>,

    /// Nonce管理器
    nonce_manager: Arc<NonceManager>,

    /// DID文档缓存
    did_cache: Arc<DIDCache>,

    /// 本地密钥对
    keypair: Arc<RwLock<Option<KeyPair>>>,

    /// 本地PeerID
    peer_id: Arc<RwLock<Option<PeerId>>>,

    /// 本地DID的CID
    local_cid: Arc<RwLock<Option<String>>>,

    /// 主题配置
    topic_configs: Arc<RwLock<HashMap<String, TopicConfig>>>,

    /// 订阅的主题列表
    subscribed_topics: Arc<RwLock<Vec<String>>>,

    /// 消息统计
    message_stats: Arc<RwLock<HashMap<String, u64>>>, // topic -> message_count
}

impl PubsubAuthenticator {
    /// 创建新的Pubsub认证器
    pub fn new(
        identity_manager: IdentityManager,
        nonce_manager: Option<NonceManager>,
        did_cache: Option<DIDCache>,
    ) -> Self {
        log::info!("🔐 创建Pubsub认证器");

        Self {
            identity_manager: Arc::new(identity_manager),
            nonce_manager: Arc::new(nonce_manager.unwrap_or_default()),
            did_cache: Arc::new(did_cache.unwrap_or_default()),
            keypair: Arc::new(RwLock::new(None)),
            peer_id: Arc::new(RwLock::new(None)),
            local_cid: Arc::new(RwLock::new(None)),
            topic_configs: Arc::new(RwLock::new(HashMap::new())),
            subscribed_topics: Arc::new(RwLock::new(Vec::new())),
            message_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 判断给定标识是否为 IPNS 格式
    fn is_ipns_format(value: &str) -> bool {
        let v = value.trim();
        if v.starts_with("/ipns/") {
            return true;
        }
        // 尝试作为 base58btc 的 PeerID 粗略校验（长度与可解码性）
        // 典型长度 46-62 字符（不同多码/多基编码可能变化，这里做宽松判断）
        if v.len() >= 46 && v.len() <= 100 {
            if bs58::decode(v).into_vec().is_ok() {
                return true;
            }
        }
        false
    }

    /// 从 DID 文档中提取 PubSub 认证主题
    pub fn extract_auth_topic_from_did(
        did_document: &crate::did_builder::DIDDocument,
    ) -> Option<String> {
        did_document
            .service
            .as_ref()
            .and_then(|services| {
                services
                    .iter()
                    .find(|svc| svc.service_type.eq_ignore_ascii_case("PubSubAuth"))
            })
            .and_then(|svc| svc.service_endpoint.get("topic"))
            .and_then(|topic| topic.as_str().map(|s| s.to_string()))
    }

    /// 构建身份认证请求消息（需要自行通过pubsub发送）
    pub async fn send_auth_request(
        &self,
        auth_topic: &str,
        target_cid: &str,
        response_topic: Option<String>,
        to_did: Option<String>,
        note: Option<String>,
    ) -> Result<AuthenticatedMessage> {
        let payload = PubsubAuthRequestPayload {
            target_cid: target_cid.to_string(),
            response_topic,
            note,
        };
        let payload_bytes = serde_json::to_vec(&payload).context("序列化认证请求负载失败")?;

        self.create_authenticated_message(
            auth_topic,
            PubSubMessageType::AuthRequest,
            &payload_bytes,
            to_did,
        )
        .await
    }

    /// 处理身份认证请求消息，返回需要发送的响应消息及负载
    pub async fn handle_auth_request(
        &self,
        request: &AuthenticatedMessage,
        override_response_topic: Option<&str>,
        note: Option<String>,
    ) -> Result<(AuthenticatedMessage, PubsubAuthResponsePayload)> {
        if request.message_type != PubSubMessageType::AuthRequest {
            anyhow::bail!("消息类型不是 AuthRequest");
        }

        let request_payload = Self::parse_auth_request(request)?;

        // 认证请求应当指向当前身份
        let local_cid = self
            .local_cid
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("未设置本地身份 CID，无法响应认证请求"))?;

        if request_payload.target_cid != local_cid {
            log::warn!(
                "收到的认证请求CID ({}) 与本地CID ({}) 不匹配",
                request_payload.target_cid,
                local_cid
            );
        }

        let response_topic = override_response_topic
            .map(|s| s.to_string())
            .or(request_payload.response_topic.clone())
            .unwrap_or_else(|| request.topic.clone());

        let response_payload = PubsubAuthResponsePayload {
            request_nonce: request.nonce.clone(),
            target_cid: local_cid.clone(),
            success: true,
            note,
        };

        let payload_bytes =
            serde_json::to_vec(&response_payload).context("序列化认证响应负载失败")?;

        let response_message = self
            .create_authenticated_message(
                &response_topic,
                PubSubMessageType::AuthResponse,
                &payload_bytes,
                Some(request.from_did.clone()),
            )
            .await?;

        Ok((response_message, response_payload))
    }

    /// 解析认证请求消息的负载
    pub fn parse_auth_request(message: &AuthenticatedMessage) -> Result<PubsubAuthRequestPayload> {
        if message.message_type != PubSubMessageType::AuthRequest {
            anyhow::bail!("消息类型不是 AuthRequest");
        }
        let payload: PubsubAuthRequestPayload =
            serde_json::from_slice(&message.content).context("解析认证请求负载失败")?;
        Ok(payload)
    }

    /// 解析认证响应消息的负载
    pub fn parse_auth_response(
        message: &AuthenticatedMessage,
    ) -> Result<PubsubAuthResponsePayload> {
        if message.message_type != PubSubMessageType::AuthResponse {
            anyhow::bail!("消息类型不是 AuthResponse");
        }
        let payload: PubsubAuthResponsePayload =
            serde_json::from_slice(&message.content).context("解析认证响应负载失败")?;
        Ok(payload)
    }

    /// 设置本地身份
    pub async fn set_local_identity(
        &self,
        keypair: KeyPair,
        peer_id: PeerId,
        cid: String,
    ) -> Result<()> {
        *self.keypair.write().await = Some(keypair);
        *self.peer_id.write().await = Some(peer_id);
        *self.local_cid.write().await = Some(cid.clone());

        log::info!("✓ 设置本地身份");
        log::info!("  CID: {}", cid);

        Ok(())
    }

    /// 配置主题策略
    pub async fn configure_topic(&self, config: TopicConfig) -> Result<()> {
        let topic_name = config.name.clone();
        self.topic_configs
            .write()
            .await
            .insert(topic_name.clone(), config);

        log::info!("✓ 配置主题: {}", topic_name);

        Ok(())
    }

    /// 创建认证消息
    pub async fn create_authenticated_message(
        &self,
        topic: &str,
        message_type: PubSubMessageType,
        content: &[u8],
        to_did: Option<String>,
    ) -> Result<AuthenticatedMessage> {
        // 1. 检查本地身份
        let keypair = self
            .keypair
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置本地身份"))?
            .clone();

        let peer_id = self
            .peer_id
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置PeerID"))?
            .to_string();

        let cid = self
            .local_cid
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置CID"))?
            .clone();

        // 2. 生成nonce
        let nonce = NonceManager::generate_nonce();

        // 3. 获取DID文档（用于ZKP证明）
        let did_document = crate::did_builder::get_did_document_from_cid(
            self.identity_manager.ipfs_client(),
            &cid,
        )
        .await?;

        // 4. 生成ZKP证明
        let zkp_proof = self.identity_manager.generate_binding_proof(
            &keypair,
            &did_document,
            &cid,
            nonce.as_bytes(),
        )?;

        // 5. 签名消息内容
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::from_bytes(&keypair.private_key);

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(content);
        sign_data.extend_from_slice(nonce.as_bytes());
        sign_data.extend_from_slice(topic.as_bytes());

        let signature = signing_key.sign(&sign_data);

        // 6. 构造认证消息
        let message = AuthenticatedMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type,
            from_did: keypair.did.clone(),
            to_did,
            from_peer_id: peer_id,
            did_cid: cid,
            topic: topic.to_string(),
            content: content.to_vec(),
            nonce,
            zkp_proof: zkp_proof,
            signature: signature.to_bytes().to_vec(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        log::debug!("✓ 创建认证消息: {}", message.message_id);

        Ok(message)
    }

    /// 验证认证消息
    pub async fn verify_message(
        &self,
        message: &AuthenticatedMessage,
    ) -> Result<MessageVerification> {
        let mut details = Vec::new();
        let mut verified = true;

        log::info!("🔍 验证消息: {}", message.message_id);
        log::info!("  发送者DID: {}", message.from_did);

        // 0. 规范化/解析 DID 标识（支持 IPNS 名称）
        let mut resolved_cid = message.did_cid.clone();
        if Self::is_ipns_format(&message.did_cid) {
            log::info!("🔎 检测到 IPNS 标识，开始解析: {}", message.did_cid);
            match self
                .identity_manager
                .ipfs_client()
                .resolve_ipns(&message.did_cid)
                .await
            {
                Ok(cid) => {
                    details.push(format!("✓ IPNS 解析成功: {} -> {}", message.did_cid, cid));
                    resolved_cid = cid;
                }
                Err(e) => {
                    details.push(format!("✗ IPNS 解析失败: {}", e));
                    return Ok(MessageVerification {
                        verified: false,
                        from_did: message.from_did.clone(),
                        details,
                        verified_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    });
                }
            }
        }

        // 1. 验证nonce（防重放）
        match self
            .nonce_manager
            .verify_and_record(&message.nonce, &message.from_did)
        {
            Ok(true) => {
                details.push("✓ Nonce验证通过".to_string());
            }
            Ok(false) => {
                verified = false;
                details.push("✗ Nonce已被使用（重放攻击）".to_string());
                log::warn!("检测到重放攻击！消息ID: {}", message.message_id);
            }
            Err(e) => {
                verified = false;
                details.push(format!("✗ Nonce验证失败: {}", e));
            }
        }

        // 2. 检查主题授权
        let topic_config = self.topic_configs.read().await;
        if let Some(config) = topic_config.get(&message.topic) {
            match &config.policy {
                TopicPolicy::AllowAuthenticated => {
                    // 通过认证即可
                }
                TopicPolicy::AllowList(allowed) => {
                    if !allowed.contains(&message.from_did) {
                        verified = false;
                        details.push(format!("✗ DID不在允许列表中"));
                    }
                }
                TopicPolicy::DenyList(denied) => {
                    if denied.contains(&message.from_did) {
                        verified = false;
                        details.push(format!("✗ DID在拒绝列表中"));
                    }
                }
                TopicPolicy::Custom => {
                    // 自定义验证逻辑
                }
            }
        }

        // 3. 获取DID文档（先从缓存）
        let did_document = if let Some(doc) = self.did_cache.get(&resolved_cid) {
            details.push("✓ 从缓存获取DID文档".to_string());
            doc
        } else {
            match crate::did_builder::get_did_document_from_cid(
                self.identity_manager.ipfs_client(),
                &resolved_cid,
            )
            .await
            {
                Ok(doc) => {
                    self.did_cache
                        .put(resolved_cid.clone(), doc.clone())
                        .ok();
                    details.push("✓ 从IPFS获取DID文档并缓存".to_string());
                    doc
                }
                Err(e) => {
                    details.push(format!("✗ 获取DID文档失败: {}", e));

                    return Ok(MessageVerification {
                        verified: false,
                        from_did: message.from_did.clone(),
                        details,
                        verified_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    });
                }
            }
        };

        // 4. 验证ZKP证明
        let zkp_result = self
            .identity_manager
            .verify_identity_with_zkp(
                &resolved_cid,
                &message.zkp_proof,
                message.nonce.as_bytes(),
            )
            .await;

        match zkp_result {
            Ok(verification) if verification.zkp_verified => {
                details.push("✓ ZKP证明验证通过".to_string());
            }
            Ok(_) => {
                verified = false;
                details.push("✗ ZKP证明验证失败".to_string());
            }
            Err(e) => {
                verified = false;
                details.push(format!("✗ ZKP验证错误: {}", e));
            }
        }

        // 5. 验证消息签名
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let public_key_bytes = self.extract_public_key(&did_document)?;
        let key_bytes = if public_key_bytes.len() > 32 {
            &public_key_bytes[public_key_bytes.len() - 32..]
        } else {
            &public_key_bytes
        };

        let verifying_key =
            VerifyingKey::from_bytes(key_bytes.try_into().context("公钥长度错误")?)?;

        let signature = Signature::from_bytes(
            message
                .signature
                .as_slice()
                .try_into()
                .context("签名长度错误")?,
        );

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&message.content);
        sign_data.extend_from_slice(message.nonce.as_bytes());
        sign_data.extend_from_slice(message.topic.as_bytes());

        match verifying_key.verify(&sign_data, &signature) {
            Ok(_) => {
                details.push("✓ 消息签名验证通过".to_string());
            }
            Err(_) => {
                verified = false;
                details.push("✗ 消息签名验证失败".to_string());
            }
        }

        log::info!("验证结果: {}", if verified { "✅ 通过" } else { "❌ 失败" });

        Ok(MessageVerification {
            verified,
            from_did: message.from_did.clone(),
            details,
            verified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }

    /// 从DID文档提取公钥
    fn extract_public_key(
        &self,
        did_document: &crate::did_builder::DIDDocument,
    ) -> Result<Vec<u8>> {
        let vm = did_document
            .verification_method
            .first()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少验证方法"))?;

        let pk_multibase = &vm.public_key_multibase;
        let pk_bs58 = pk_multibase.trim_start_matches('z');
        let public_key = bs58::decode(pk_bs58).into_vec().context("解码公钥失败")?;

        Ok(public_key)
    }

    /// 序列化消息为字节
    pub fn serialize_message(message: &AuthenticatedMessage) -> Result<Vec<u8>> {
        bincode::serialize(message).context("序列化消息失败")
    }

    /// 反序列化消息
    pub fn deserialize_message(data: &[u8]) -> Result<AuthenticatedMessage> {
        bincode::deserialize(data).context("反序列化消息失败")
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> crate::did_cache::CacheStats {
        self.did_cache.stats()
    }

    /// 获取nonce统计
    pub fn nonce_count(&self) -> usize {
        self.nonce_manager.count()
    }

    /// 订阅主题
    pub async fn subscribe_topic(&self, topic: &str) -> Result<()> {
        let mut topics = self.subscribed_topics.write().await;
        if !topics.contains(&topic.to_string()) {
            topics.push(topic.to_string());
            log::info!("✓ 订阅主题: {}", topic);
        }
        Ok(())
    }

    /// 取消订阅主题
    pub async fn unsubscribe_topic(&self, topic: &str) -> Result<()> {
        let mut topics = self.subscribed_topics.write().await;
        topics.retain(|t| t != topic);
        log::info!("✓ 取消订阅主题: {}", topic);
        Ok(())
    }

    /// 获取订阅的主题列表
    pub async fn get_subscribed_topics(&self) -> Vec<String> {
        self.subscribed_topics.read().await.clone()
    }

    /// 更新消息统计
    pub async fn update_message_stats(&self, topic: &str) {
        let mut stats = self.message_stats.write().await;
        *stats.entry(topic.to_string()).or_insert(0) += 1;
    }

    /// 获取消息统计
    pub async fn get_message_stats(&self) -> HashMap<String, u64> {
        self.message_stats.read().await.clone()
    }

    /// 创建简化的认证消息（用于演示）
    pub async fn create_simple_message(
        &self,
        topic: &str,
        content: &str,
    ) -> Result<AuthenticatedMessage> {
        self.create_authenticated_message(
            topic,
            PubSubMessageType::Custom("simple_message".to_string()),
            content.as_bytes(),
            None,
        )
        .await
    }

    /// 创建身份验证请求消息
    pub async fn create_auth_request(
        &self,
        topic: &str,
        target_did: &str,
        challenge: &str,
    ) -> Result<AuthenticatedMessage> {
        let content = format!("AUTH_REQUEST:{}:{}", target_did, challenge);
        self.create_authenticated_message(
            topic,
            PubSubMessageType::AuthRequest,
            content.as_bytes(),
            Some(target_did.to_string()),
        )
        .await
    }

    /// 创建身份验证响应消息
    pub async fn create_auth_response(
        &self,
        topic: &str,
        target_did: &str,
        response: &str,
    ) -> Result<AuthenticatedMessage> {
        let content = format!("AUTH_RESPONSE:{}:{}", target_did, response);
        self.create_authenticated_message(
            topic,
            PubSubMessageType::AuthResponse,
            content.as_bytes(),
            Some(target_did.to_string()),
        )
        .await
    }

    /// 创建心跳消息
    pub async fn create_heartbeat(&self, topic: &str) -> Result<AuthenticatedMessage> {
        let content = format!(
            "HEARTBEAT:{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        self.create_authenticated_message(
            topic,
            PubSubMessageType::Heartbeat,
            content.as_bytes(),
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要实际的IPFS和ZKP设置
    async fn test_create_authenticated_message() {
        // 这个测试需要完整的环境设置
        // 包括IPFS客户端、ZKP keys等
    }
}
