// DIAP Rust SDK - IPFS Pubsub认证通讯模块
// 基于libp2p gossipsub实现认证的发布/订阅通信

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use libp2p::PeerId;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::identity_manager::IdentityManager;
use crate::key_manager::KeyPair;
use crate::nonce_manager::NonceManager;
use crate::did_cache::DIDCache;

/// PubSub消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        self.topic_configs.write().await.insert(topic_name.clone(), config);
        
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
        let keypair = self.keypair.read().await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置本地身份"))?
            .clone();
        
        let peer_id = self.peer_id.read().await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置PeerID"))?
            .to_string();
        
        let cid = self.local_cid.read().await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未设置CID"))?
            .clone();
        
        // 2. 生成nonce
        let nonce = NonceManager::generate_nonce();
        
        // 3. 获取DID文档（用于ZKP证明）
        let did_document = crate::did_builder::get_did_document_from_cid(
            self.identity_manager.ipfs_client(),
            &cid
        ).await?;
        
        // 4. 生成ZKP证明
        let zkp_proof = self.identity_manager.generate_binding_proof(
            &keypair,
            &did_document,
            &cid,
            nonce.as_bytes(),
        )?;
        
        // 5. 签名消息内容
        use ed25519_dalek::{SigningKey, Signer};
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
        
        // 1. 验证nonce（防重放）
        match self.nonce_manager.verify_and_record(&message.nonce, &message.from_did) {
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
        let did_document = if let Some(doc) = self.did_cache.get(&message.did_cid) {
            details.push("✓ 从缓存获取DID文档".to_string());
            doc
        } else {
            match crate::did_builder::get_did_document_from_cid(
                self.identity_manager.ipfs_client(),
                &message.did_cid
            ).await {
                Ok(doc) => {
                    self.did_cache.put(message.did_cid.clone(), doc.clone()).ok();
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
        let zkp_result = self.identity_manager.verify_identity_with_zkp(
            &message.did_cid,
            &message.zkp_proof,
            message.nonce.as_bytes(),
        ).await;
        
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
        use ed25519_dalek::{VerifyingKey, Verifier, Signature};
        
        let public_key_bytes = self.extract_public_key(&did_document)?;
        let key_bytes = if public_key_bytes.len() > 32 {
            &public_key_bytes[public_key_bytes.len() - 32..]
        } else {
            &public_key_bytes
        };
        
        let verifying_key = VerifyingKey::from_bytes(
            key_bytes.try_into().context("公钥长度错误")?
        )?;
        
        let signature = Signature::from_bytes(
            message.signature.as_slice().try_into().context("签名长度错误")?
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
    fn extract_public_key(&self, did_document: &crate::did_builder::DIDDocument) -> Result<Vec<u8>> {
        let vm = did_document.verification_method.first()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少验证方法"))?;
        
        let pk_multibase = &vm.public_key_multibase;
        let pk_bs58 = pk_multibase.trim_start_matches('z');
        let public_key = bs58::decode(pk_bs58).into_vec()
            .context("解码公钥失败")?;
        
        Ok(public_key)
    }
    
    /// 序列化消息为字节（bincode 2.x + serde feature: bincode::serde:: 命名空间）
    pub fn serialize_message(message: &AuthenticatedMessage) -> Result<Vec<u8>> {
        bincode::serde::encode_to_vec(message, bincode::config::standard())
            .map_err(|e| anyhow::anyhow!("序列化消息失败: {e}"))
    }

    /// 反序列化消息（bincode 2.x + serde feature）
    pub fn deserialize_message(data: &[u8]) -> Result<AuthenticatedMessage> {
        let (msg, _): (AuthenticatedMessage, usize) =
            bincode::serde::decode_from_slice(data, bincode::config::standard())
                .map_err(|e| anyhow::anyhow!("反序列化消息失败: {e}"))?;
        Ok(msg)
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
        ).await
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
        ).await
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
        ).await
    }
    
    /// 创建心跳消息
    pub async fn create_heartbeat(&self, topic: &str) -> Result<AuthenticatedMessage> {
        let content = format!("HEARTBEAT:{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
        
        self.create_authenticated_message(
            topic,
            PubSubMessageType::Heartbeat,
            content.as_bytes(),
            None,
        ).await
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

