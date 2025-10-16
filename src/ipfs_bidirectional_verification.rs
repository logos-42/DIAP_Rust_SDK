// DIAP Rust SDK - IPFS双向验证系统
// 实现基于真实IPFS的智能体双向身份验证闭环

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{timeout, Duration};

use crate::{
    IpfsClient, IpfsNodeManager, IpfsNodeConfig,
    KeyPair, DIDDocument, AgentInfo,
    AgentVerificationManager, AgentVerificationRequest,
};

/// IPFS双向验证管理器
pub struct IpfsBidirectionalVerificationManager {
    /// IPFS客户端
    ipfs_client: IpfsClient,
    /// IPFS节点管理器
    ipfs_node_manager: IpfsNodeManager,
    /// 智能体验证管理器
    verification_manager: AgentVerificationManager,
    /// 活跃的智能体会话
    active_sessions: HashMap<String, AgentSession>,
    /// 验证缓存
    verification_cache: HashMap<String, VerificationResult>,
}

/// 智能体会话
#[derive(Debug, Clone)]
pub struct AgentSession {
    /// 智能体ID
    pub agent_id: String,
    /// 智能体信息
    pub agent_info: AgentInfo,
    /// 密钥对
    pub keypair: KeyPair,
    /// DID文档CID
    pub did_document_cid: String,
    /// 会话创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 会话状态
    pub status: SessionStatus,
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    /// 等待验证
    Pending,
    /// 验证中
    Verifying,
    /// 已验证
    Verified,
    /// 验证失败
    Failed,
    /// 已过期
    Expired,
}

/// 双向验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidirectionalVerificationResult {
    /// 验证是否成功
    pub success: bool,
    /// 发起方智能体ID
    pub initiator_id: String,
    /// 响应方智能体ID
    pub responder_id: String,
    /// 发起方验证结果
    pub initiator_result: VerificationResult,
    /// 响应方验证结果
    pub responder_result: VerificationResult,
    /// 验证时间戳
    pub verification_timestamp: u64,
    /// 总验证时间（毫秒）
    pub total_verification_time_ms: u64,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// 智能体ID
    pub agent_id: String,
    /// 验证状态
    pub status: VerificationStatus,
    /// 证明数据
    pub proof: Option<ProofData>,
    /// 验证时间戳
    pub timestamp: u64,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}

/// 验证状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 网络错误
    NetworkError,
    /// 数据错误
    DataError,
}

/// 证明数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofData {
    /// 证明内容
    pub proof: Vec<u8>,
    /// 公共输入
    pub public_inputs: Vec<u8>,
    /// 电路输出
    pub circuit_output: String,
    /// DID文档内容
    pub did_document_content: String,
    /// 资源CID
    pub resource_cid: String,
    /// 挑战nonce
    pub challenge_nonce: String,
}

/// 验证挑战
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationChallenge {
    /// 挑战ID
    pub challenge_id: String,
    /// 发起方智能体ID
    pub initiator_id: String,
    /// 响应方智能体ID
    pub responder_id: String,
    /// 挑战nonce
    pub challenge_nonce: String,
    /// 挑战时间戳
    pub timestamp: u64,
    /// 过期时间（秒）
    pub expiry_seconds: u64,
    /// 资源CID
    pub resource_cid: String,
}

impl IpfsBidirectionalVerificationManager {
    /// 创建新的双向验证管理器
    pub async fn new() -> Result<Self> {
        log::info!("🚀 初始化IPFS双向验证管理器");
        
        // 配置IPFS节点
        let ipfs_config = IpfsNodeConfig {
            data_dir: std::env::temp_dir().join("diap_bidirectional_verification"),
            api_port: 5001,
            gateway_port: 8080,
            auto_start: true,
            startup_timeout: 30,
            enable_bootstrap: true,
            enable_swarm: true,
            swarm_port: 4001,
            verbose_logging: false,
        };
        
        // 创建IPFS客户端和节点管理器
        let (ipfs_client, ipfs_node_manager) = IpfsClient::new_builtin_only(
            Some(ipfs_config.clone()),
            30
        ).await?;
        
        // 创建智能体验证管理器
        let verification_manager = AgentVerificationManager::new("./noir_circuits".to_string());
        
        Ok(Self {
            ipfs_client,
            ipfs_node_manager,
            verification_manager,
            active_sessions: HashMap::new(),
            verification_cache: HashMap::new(),
        })
    }
    
    /// 注册智能体到IPFS网络
    pub async fn register_agent(
        &mut self,
        agent_info: &AgentInfo,
        keypair: &KeyPair,
    ) -> Result<String> {
        log::info!("📝 注册智能体到IPFS网络: {}", agent_info.name);
        
        // 创建DID文档
        let did_document = self.create_did_document(agent_info, keypair)?;
        let did_doc_json = serde_json::to_string_pretty(&did_document)?;
        
        // 上传DID文档到IPFS
        let upload_result = self.ipfs_client.upload(&did_doc_json, &format!("{}.json", agent_info.name)).await?;
        
        log::info!("✅ 智能体注册成功");
        log::info!("   DID文档CID: {}", upload_result.cid);
        log::info!("   文档大小: {} bytes", upload_result.size);
        log::info!("   上传提供商: {}", upload_result.provider);
        
        // 创建智能体会话
        let session = AgentSession {
            agent_id: agent_info.name.clone(),
            agent_info: agent_info.clone(),
            keypair: keypair.clone(),
            did_document_cid: upload_result.cid.clone(),
            created_at: self.get_current_timestamp(),
            last_activity: self.get_current_timestamp(),
            status: SessionStatus::Pending,
        };
        
        // 保存会话
        self.active_sessions.insert(agent_info.name.clone(), session);
        
        Ok(upload_result.cid)
    }
    
    /// 发起双向验证
    pub async fn initiate_bidirectional_verification(
        &mut self,
        initiator_id: &str,
        responder_id: &str,
        resource_cid: &str,
    ) -> Result<BidirectionalVerificationResult> {
        let start_time = std::time::Instant::now();
        log::info!("🤝 发起双向验证: {} ↔ {}", initiator_id, responder_id);
        
        // 检查智能体是否已注册并克隆必要数据
        let initiator_session = self.active_sessions.get(initiator_id)
            .ok_or_else(|| anyhow::anyhow!("发起方智能体未注册: {}", initiator_id))?
            .clone();
        
        let responder_session = self.active_sessions.get(responder_id)
            .ok_or_else(|| anyhow::anyhow!("响应方智能体未注册: {}", responder_id))?
            .clone();
        
        // 创建验证挑战
        let challenge = VerificationChallenge {
            challenge_id: format!("{}-{}-{}", initiator_id, responder_id, self.get_current_timestamp()),
            initiator_id: initiator_id.to_string(),
            responder_id: responder_id.to_string(),
            challenge_nonce: format!("challenge_{}_{}", initiator_id, responder_id),
            timestamp: self.get_current_timestamp(),
            expiry_seconds: 300, // 5分钟过期
            resource_cid: resource_cid.to_string(),
        };
        
        // 顺序执行双向验证（因为需要可变借用）
        let initiator_result = self.verify_agent_identity(
            &initiator_session,
            &challenge,
            &responder_session.did_document_cid,
        ).await?;
        
        let responder_result = self.verify_agent_identity(
            &responder_session,
            &challenge,
            &initiator_session.did_document_cid,
        ).await?;
        
        let total_time = start_time.elapsed().as_millis() as u64;
        
        // 判断验证是否成功
        let success = matches!(initiator_result.status, VerificationStatus::Success) &&
                     matches!(responder_result.status, VerificationStatus::Success);
        
        let result = BidirectionalVerificationResult {
            success,
            initiator_id: initiator_id.to_string(),
            responder_id: responder_id.to_string(),
            initiator_result,
            responder_result,
            verification_timestamp: self.get_current_timestamp(),
            total_verification_time_ms: total_time,
            error_message: if success { None } else { Some("双向验证失败".to_string()) },
        };
        
        // 缓存验证结果
        let cache_key = format!("{}-{}-{}", initiator_id, responder_id, resource_cid);
        self.verification_cache.insert(cache_key, result.initiator_result.clone());
        
        if success {
            log::info!("✅ 双向验证成功完成");
        } else {
            log::warn!("❌ 双向验证失败");
        }
        
        Ok(result)
    }
    
    /// 验证单个智能体身份
    async fn verify_agent_identity(
        &mut self,
        agent_session: &AgentSession,
        challenge: &VerificationChallenge,
        peer_did_cid: &str,
    ) -> Result<VerificationResult> {
        let start_time = std::time::Instant::now();
        log::info!("🔍 验证智能体身份: {}", agent_session.agent_id);
        
        // 从IPFS获取对等方的DID文档
        let peer_did_document = match self.ipfs_client.get(peer_did_cid).await {
            Ok(content) => content,
            Err(e) => {
                log::error!("❌ 无法从IPFS获取DID文档: {}", e);
                return Ok(VerificationResult {
                    agent_id: agent_session.agent_id.clone(),
                    status: VerificationStatus::NetworkError,
                    proof: None,
                    timestamp: self.get_current_timestamp(),
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(format!("无法获取DID文档: {}", e)),
                });
            }
        };
        
        // 创建验证请求
        let verification_request = AgentVerificationRequest {
            agent_id: agent_session.agent_id.clone(),
            resource_cid: challenge.resource_cid.clone(),
            challenge_nonce: challenge.challenge_nonce.clone(),
            timestamp: challenge.timestamp,
            expiry_seconds: challenge.expiry_seconds,
        };
        
        // 执行智能体验证
        let verification_response = match timeout(
            Duration::from_secs(30),
            self.verification_manager.verify_agent_access(
                &verification_request,
                &agent_session.keypair.private_key,
                &peer_did_document,
            )
        ).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                log::error!("❌ 智能体验证失败: {}", e);
                return Ok(VerificationResult {
                    agent_id: agent_session.agent_id.clone(),
                    status: VerificationStatus::Failed,
                    proof: None,
                    timestamp: self.get_current_timestamp(),
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(format!("智能体验证失败: {}", e)),
                });
            }
            Err(_) => {
                log::error!("❌ 验证超时");
                return Ok(VerificationResult {
                    agent_id: agent_session.agent_id.clone(),
                    status: VerificationStatus::Timeout,
                    proof: None,
                    timestamp: self.get_current_timestamp(),
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some("验证超时".to_string()),
                });
            }
        };
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        // 创建证明数据
        let proof_data = if let (Some(proof), Some(public_inputs), Some(circuit_output)) = 
            (&verification_response.proof, &verification_response.public_inputs, &verification_response.circuit_output) {
            
            Some(ProofData {
                proof: proof.clone(),
                public_inputs: public_inputs.clone(),
                circuit_output: circuit_output.clone(),
                did_document_content: peer_did_document,
                resource_cid: challenge.resource_cid.clone(),
                challenge_nonce: challenge.challenge_nonce.clone(),
            })
        } else {
            None
        };
        
        // 确定验证状态
        let status = match verification_response.status {
            crate::AgentVerificationStatus::Verified => VerificationStatus::Success,
            crate::AgentVerificationStatus::Failed => VerificationStatus::Failed,
            crate::AgentVerificationStatus::Expired => VerificationStatus::Timeout,
            _ => VerificationStatus::Failed,
        };
        
        let result = VerificationResult {
            agent_id: agent_session.agent_id.clone(),
            status,
            proof: proof_data,
            timestamp: self.get_current_timestamp(),
            processing_time_ms: processing_time,
            error_message: verification_response.error_message,
        };
        
        if matches!(result.status, VerificationStatus::Success) {
            log::info!("✅ 智能体身份验证成功: {}", agent_session.agent_id);
        } else {
            log::warn!("❌ 智能体身份验证失败: {}", agent_session.agent_id);
        }
        
        Ok(result)
    }
    
    /// 批量验证多个智能体对
    pub async fn batch_bidirectional_verification(
        &mut self,
        agent_pairs: Vec<(String, String)>,
        resource_cid: &str,
    ) -> Result<Vec<BidirectionalVerificationResult>> {
        log::info!("🔄 开始批量双向验证: {} 对智能体", agent_pairs.len());
        
        let mut results = Vec::new();
        let mut success_count = 0;
        
        for (initiator_id, responder_id) in agent_pairs {
            match self.initiate_bidirectional_verification(&initiator_id, &responder_id, resource_cid).await {
                Ok(result) => {
                    if result.success {
                        success_count += 1;
                    }
                    results.push(result);
                }
                Err(e) => {
                    log::error!("❌ 批量验证失败 {} ↔ {}: {}", initiator_id, responder_id, e);
                    // 创建失败的验证结果
                    results.push(BidirectionalVerificationResult {
                        success: false,
                        initiator_id: initiator_id.clone(),
                        responder_id: responder_id.clone(),
                        initiator_result: VerificationResult {
                            agent_id: initiator_id,
                            status: VerificationStatus::Failed,
                            proof: None,
                            timestamp: self.get_current_timestamp(),
                            processing_time_ms: 0,
                            error_message: Some(e.to_string()),
                        },
                        responder_result: VerificationResult {
                            agent_id: responder_id,
                            status: VerificationStatus::Failed,
                            proof: None,
                            timestamp: self.get_current_timestamp(),
                            processing_time_ms: 0,
                            error_message: Some(e.to_string()),
                        },
                        verification_timestamp: self.get_current_timestamp(),
                        total_verification_time_ms: 0,
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
        
        log::info!("✅ 批量双向验证完成: {}/{} 成功", success_count, results.len());
        Ok(results)
    }
    
    /// 获取智能体会话信息
    pub fn get_agent_session(&self, agent_id: &str) -> Option<&AgentSession> {
        self.active_sessions.get(agent_id)
    }
    
    /// 获取所有活跃会话
    pub fn get_active_sessions(&self) -> &HashMap<String, AgentSession> {
        &self.active_sessions
    }
    
    /// 清理过期会话
    pub fn cleanup_expired_sessions(&mut self) {
        let current_time = self.get_current_timestamp();
        let mut expired_agents = Vec::new();
        
        for (agent_id, session) in &self.active_sessions {
            // 会话超过1小时未活动则过期
            if current_time - session.last_activity > 3600 {
                expired_agents.push(agent_id.clone());
            }
        }
        
        let expired_count = expired_agents.len();
        for agent_id in expired_agents {
            if let Some(session) = self.active_sessions.get_mut(&agent_id) {
                session.status = SessionStatus::Expired;
            }
        }
        
        log::info!("🧹 清理了 {} 个过期会话", expired_count);
    }
    
    /// 获取IPFS节点状态
    pub async fn get_ipfs_node_status(&self) -> Result<String> {
        match self.ipfs_node_manager.get_node_info().await {
            Ok(info) => {
                Ok(format!("节点ID: {}, 版本: {}", info.id, info.agent_version))
            }
            Err(e) => {
                Ok(format!("节点状态未知: {}", e))
            }
        }
    }
    
    /// 获取IPFS客户端（用于共享访问）
    pub fn get_ipfs_client(&self) -> IpfsClient {
        self.ipfs_client.clone()
    }
    
    // 私有辅助方法
    
    /// 创建DID文档
    fn create_did_document(&self, agent_info: &AgentInfo, keypair: &KeyPair) -> Result<DIDDocument> {
        // 创建验证方法
        let verification_method = crate::VerificationMethod {
            id: format!("{}#key-1", keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: keypair.did.clone(),
            public_key_multibase: format!("z{}", bs58::encode(&keypair.public_key).into_string()),
        };
        
        Ok(DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: keypair.did.clone(),
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id.clone()],
            service: Some(vec![crate::Service {
                id: format!("{}#service", keypair.did),
                service_type: "DIAP Agent Service".to_string(),
                service_endpoint: format!("https://{}.example.com", agent_info.name.to_lowercase()).into(),
                pubsub_topics: None,
                network_addresses: None,
            }]),
            created: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verification_manager_creation() {
        // 注意：这个测试需要IPFS环境
        let result = IpfsBidirectionalVerificationManager::new().await;
        if result.is_ok() {
            println!("✅ 双向验证管理器创建成功");
        } else {
            println!("⚠️  双向验证管理器创建失败（可能是IPFS未安装）: {:?}", result.err());
        }
    }
}