// DIAP Rust SDK - 智能体验证闭环
// 使用Noir电路实现完整的智能体验证流程

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 智能体验证状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentVerificationStatus {
    /// 待验证
    Pending,
    /// 验证中
    Verifying,
    /// 验证成功
    Verified,
    /// 验证失败
    Failed,
    /// 已过期
    Expired,
}

/// 智能体验证请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVerificationRequest {
    /// 智能体ID
    pub agent_id: String,
    /// 资源CID
    pub resource_cid: String,
    /// 挑战nonce
    pub challenge_nonce: String,
    /// 请求时间戳
    pub timestamp: u64,
    /// 过期时间（秒）
    pub expiry_seconds: u64,
}

/// 智能体验证响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVerificationResponse {
    /// 验证状态
    pub status: AgentVerificationStatus,
    /// 证明数据
    pub proof: Option<Vec<u8>>,
    /// 公共输入
    pub public_inputs: Option<Vec<u8>>,
    /// 电路输出
    pub circuit_output: Option<String>,
    /// 验证时间戳
    pub verification_timestamp: u64,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 智能体验证管理器
pub struct AgentVerificationManager {
    /// Noir电路路径
    noir_circuits_path: String,
    /// 验证记录缓存
    verification_cache: std::collections::HashMap<String, AgentVerificationResponse>,
}

impl AgentVerificationManager {
    /// 创建新的验证管理器
    pub fn new(noir_circuits_path: String) -> Self {
        Self {
            noir_circuits_path,
            verification_cache: std::collections::HashMap::new(),
        }
    }

    /// 验证智能体访问权限
    pub async fn verify_agent_access(
        &mut self,
        request: &AgentVerificationRequest,
        agent_private_key: &[u8],
        agent_did_document: &str,
    ) -> Result<AgentVerificationResponse> {
        log::info!("🔍 开始验证智能体访问权限: {}", request.agent_id);
        
        // 检查请求是否过期
        if self.is_request_expired(request) {
            return Ok(AgentVerificationResponse {
                status: AgentVerificationStatus::Expired,
                proof: None,
                public_inputs: None,
                circuit_output: None,
                verification_timestamp: self.get_current_timestamp(),
                error_message: Some("验证请求已过期".to_string()),
            });
        }

        // 检查缓存
        let cache_key = self.generate_cache_key(request);
        if let Some(cached_response) = self.verification_cache.get(&cache_key) {
            log::info!("📦 使用缓存的验证结果");
            return Ok(cached_response.clone());
        }

        // 生成ZKP证明
        match self.generate_zkp_proof(request, agent_private_key, agent_did_document).await {
            Ok(proof_data) => {
                let response = AgentVerificationResponse {
                    status: AgentVerificationStatus::Verified,
                    proof: Some(proof_data.proof),
                    public_inputs: Some(proof_data.public_inputs),
                    circuit_output: Some(proof_data.circuit_output),
                    verification_timestamp: self.get_current_timestamp(),
                    error_message: None,
                };

                // 缓存结果
                self.verification_cache.insert(cache_key, response.clone());
                
                log::info!("✅ 智能体验证成功");
                Ok(response)
            }
            Err(e) => {
                log::error!("❌ 智能体验证失败: {}", e);
                Ok(AgentVerificationResponse {
                    status: AgentVerificationStatus::Failed,
                    proof: None,
                    public_inputs: None,
                    circuit_output: None,
                    verification_timestamp: self.get_current_timestamp(),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// 验证智能体证明
    pub async fn verify_agent_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        circuit_output: &str,
    ) -> Result<bool> {
        log::info!("🔍 验证智能体证明");
        
        // 使用Noir验证器
        use crate::noir_verifier::ImprovedNoirZKPManager;
        
        let verifier = ImprovedNoirZKPManager::new(self.noir_circuits_path.clone());
        let result = verifier.verify_proof(proof, public_inputs, circuit_output).await?;
        
        if result.is_valid {
            log::info!("✅ 证明验证成功");
        } else {
            log::warn!("❌ 证明验证失败");
            if let Some(error) = result.error_message {
                log::warn!("   错误: {}", error);
            }
        }
        
        Ok(result.is_valid)
    }

    /// 批量验证智能体
    pub async fn batch_verify_agents(
        &mut self,
        requests: Vec<AgentVerificationRequest>,
        agent_data: std::collections::HashMap<String, (Vec<u8>, String)>, // agent_id -> (private_key, did_document)
    ) -> Result<Vec<AgentVerificationResponse>> {
        log::info!("🔄 开始批量验证 {} 个智能体", requests.len());
        
        let mut responses = Vec::new();
        let mut success_count = 0;
        
        for request in requests {
            if let Some((private_key, did_document)) = agent_data.get(&request.agent_id) {
                match self.verify_agent_access(&request, private_key, did_document).await {
                    Ok(response) => {
                        if matches!(response.status, AgentVerificationStatus::Verified) {
                            success_count += 1;
                        }
                        responses.push(response);
                    }
                    Err(e) => {
                        log::error!("批量验证失败 {}: {}", request.agent_id, e);
                        responses.push(AgentVerificationResponse {
                            status: AgentVerificationStatus::Failed,
                            proof: None,
                            public_inputs: None,
                            circuit_output: None,
                            verification_timestamp: self.get_current_timestamp(),
                            error_message: Some(e.to_string()),
                        });
                    }
                }
            } else {
                log::warn!("⚠️  未找到智能体数据: {}", request.agent_id);
                responses.push(AgentVerificationResponse {
                    status: AgentVerificationStatus::Failed,
                    proof: None,
                    public_inputs: None,
                    circuit_output: None,
                    verification_timestamp: self.get_current_timestamp(),
                    error_message: Some("未找到智能体数据".to_string()),
                });
            }
        }
        
        log::info!("✅ 批量验证完成: {}/{} 成功", success_count, responses.len());
        Ok(responses)
    }

    /// 清理过期缓存
    pub fn cleanup_expired_cache(&mut self) {
        let current_time = self.get_current_timestamp();
        let mut expired_keys = Vec::new();
        
        for (key, response) in &self.verification_cache {
            // 假设缓存有效期1小时
            if current_time - response.verification_timestamp > 3600 {
                expired_keys.push(key.clone());
            }
        }
        
        let expired_count = expired_keys.len();
        for key in expired_keys {
            self.verification_cache.remove(&key);
        }
        
        log::info!("🧹 清理了 {} 个过期缓存", expired_count);
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> CacheStats {
        let total = self.verification_cache.len();
        let verified = self.verification_cache.values()
            .filter(|r| matches!(r.status, AgentVerificationStatus::Verified))
            .count();
        let failed = self.verification_cache.values()
            .filter(|r| matches!(r.status, AgentVerificationStatus::Failed))
            .count();
        
        CacheStats {
            total_entries: total,
            verified_count: verified,
            failed_count: failed,
            success_rate: if total > 0 { verified as f64 / total as f64 } else { 0.0 },
        }
    }

    // 私有方法

    /// 生成ZKP证明
    async fn generate_zkp_proof(
        &self,
        request: &AgentVerificationRequest,
        agent_private_key: &[u8],
        agent_did_document: &str,
    ) -> Result<ZKPProofData> {
        use crate::noir_zkp::NoirZKPManager;
        use crate::KeyPair;
        use crate::DIDDocument;
        
        // 创建Noir ZKP管理器
        let mut noir_manager = NoirZKPManager::new(self.noir_circuits_path.clone());
        
        // 创建KeyPair
        if agent_private_key.len() != 32 {
            anyhow::bail!("私钥长度必须是32字节");
        }
        let mut secret_key_array = [0u8; 32];
        secret_key_array.copy_from_slice(&agent_private_key[..32]);
        let keypair = KeyPair::from_private_key(secret_key_array)?;
        
        // 解析DID文档或创建默认文档
        let did_document = if !agent_did_document.is_empty() {
            serde_json::from_str::<DIDDocument>(agent_did_document)
                .unwrap_or_else(|_| self.create_default_did_document(&keypair.did))
        } else {
            self.create_default_did_document(&keypair.did)
        };
        
        // 准备输入数据
        let cid_hash = self.hash_to_bytes(request.resource_cid.as_bytes());
        let nonce = request.challenge_nonce.as_bytes().to_vec();
        
        // 生成证明
        let result = noir_manager.generate_did_binding_proof(
            &keypair,
            &did_document,
            &cid_hash,
            &nonce,
        ).await?;
        
        Ok(ZKPProofData {
            proof: result.proof,
            public_inputs: result.public_inputs,
            circuit_output: result.circuit_output,
        })
    }

    /// 创建默认DID文档
    fn create_default_did_document(&self, did: &str) -> crate::DIDDocument {
        crate::DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: did.to_string(),
            verification_method: vec![],
            authentication: vec![],
            service: None,
            created: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 哈希转字节
    fn hash_to_bytes(&self, data: &[u8]) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        
        hash.to_le_bytes().to_vec()
    }

    /// 检查请求是否过期
    fn is_request_expired(&self, request: &AgentVerificationRequest) -> bool {
        let current_time = self.get_current_timestamp();
        current_time > request.timestamp + request.expiry_seconds
    }

    /// 生成缓存键
    fn generate_cache_key(&self, request: &AgentVerificationRequest) -> String {
        format!("{}:{}:{}", request.agent_id, request.resource_cid, request.challenge_nonce)
    }

    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// ZKP证明数据
#[derive(Debug, Clone)]
struct ZKPProofData {
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    circuit_output: String,
}

/// 缓存统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub verified_count: usize,
    pub failed_count: usize,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_request_creation() {
        let request = AgentVerificationRequest {
            agent_id: "agent_001".to_string(),
            resource_cid: "QmTestResource".to_string(),
            challenge_nonce: "challenge_123".to_string(),
            timestamp: 1234567890,
            expiry_seconds: 3600,
        };
        
        assert_eq!(request.agent_id, "agent_001");
        assert_eq!(request.resource_cid, "QmTestResource");
    }

    #[tokio::test]
    async fn test_verification_manager_creation() {
        let manager = AgentVerificationManager::new("./noir_circuits".to_string());
        assert_eq!(manager.verification_cache.len(), 0);
    }
}
