use crate::{
    IdentityManager, AgentInfo, ServiceInfo, KeyPair,
    IpfsNodeManager, IpfsNodeConfig, IdentityRegistration
};
use libp2p_identity::PeerId;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use serde::{Serialize, Deserialize};

/// 智能体认证管理器 - 统一的API接口
pub struct AgentAuthManager {
    identity_manager: IdentityManager,
    ipfs_node_manager: IpfsNodeManager,
}

/// 认证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub agent_id: String,
    pub proof: Option<crate::ProofResult>,
    pub verification_details: Vec<String>,
    pub timestamp: u64,
    pub processing_time_ms: u64,
}

/// 批量认证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAuthResult {
    pub total_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub success_rate: f64,
    pub total_time_ms: u64,
    pub average_time_ms: u64,
    pub results: Vec<AuthResult>,
}

impl AgentAuthManager {
    /// 创建新的智能体认证管理器
    pub async fn new() -> Result<Self> {
        log::info!("🚀 初始化智能体认证管理器");
        
        // 配置IPFS节点
        let ipfs_config = IpfsNodeConfig {
            data_dir: std::env::temp_dir().join("diap_agent_auth"),
            api_port: 5001,
            gateway_port: 8081,
            auto_start: true,
            startup_timeout: 30,
            enable_bootstrap: true,
            enable_swarm: true,
            swarm_port: 4001,
            verbose_logging: false,
        };
        
        // 创建内置IPFS客户端（会自动启动节点）
        let (ipfs_client, ipfs_node_manager) = crate::IpfsClient::new_builtin_only(
            Some(ipfs_config.clone()),
            30
        ).await?;
        
        // 确保密钥文件存在
        let pk_path = "zkp_proving.key";
        let vk_path = "zkp_verifying.key";
        
        // 直接使用arkworks-rs生成密钥
        crate::key_generator::ensure_zkp_keys_exist(pk_path, vk_path)?;
        
        let identity_manager = IdentityManager::new_with_keys(
            ipfs_client,
            pk_path,
            vk_path
        )?;
        
        Ok(Self {
            identity_manager,
            ipfs_node_manager,
        })
    }
    
    /// 创建智能体
    pub fn create_agent(&self, name: &str, _email: Option<&str>) -> Result<(AgentInfo, KeyPair, PeerId)> {
        log::info!("🤖 创建智能体: {}", name);
        
        let agent_info = AgentInfo {
            name: name.to_string(),
            services: vec![
                ServiceInfo {
                    service_type: "messaging".to_string(),
                    endpoint: serde_json::json!(format!("https://{}.example.com/messaging", name.to_lowercase())),
                }
            ],
            description: Some(format!("{}智能体", name)),
            tags: Some(vec!["agent".to_string(), name.to_lowercase()]),
        };
        
        let keypair = KeyPair::generate()?;
        let peer_id = PeerId::random();
        
        log::info!("✅ 智能体创建成功: {}", name);
        log::info!("   DID: {}", keypair.did);
        
        Ok((agent_info, keypair, peer_id))
    }
    
    /// 注册智能体身份
    pub async fn register_agent(&self, agent_info: &AgentInfo, keypair: &KeyPair, peer_id: &PeerId) -> Result<IdentityRegistration> {
        log::info!("📝 注册智能体身份: {}", agent_info.name);
        
        let start_time = Instant::now();
        let registration = self.identity_manager.register_identity(agent_info, keypair, peer_id).await?;
        let processing_time = start_time.elapsed();
        
        log::info!("✅ 身份注册成功");
        log::info!("   CID: {}", registration.cid);
        log::info!("   注册时间: {:?}", processing_time);
        
        Ok(registration)
    }
    
    /// 生成身份证明
    pub async fn generate_proof(&self, keypair: &KeyPair, cid: &str) -> Result<AuthResult> {
        log::info!("🔐 生成身份证明");
        
        let start_time = Instant::now();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // 创建nonce
        let nonce = format!("proof_{}_{}", keypair.did, timestamp).into_bytes();
        
        // 获取DID文档
        let did_document = crate::get_did_document_from_cid(&self.identity_manager.ipfs_client(), cid).await?;
        
        // 生成证明
        let proof = self.identity_manager.generate_binding_proof(
            keypair,
            &did_document,
            cid,
            &nonce
        )?;
        
        let processing_time = start_time.elapsed();
        
        let result = AuthResult {
            success: true,
            agent_id: keypair.did.clone(),
            proof: Some(proof.clone()),
            verification_details: vec![
                "✓ 证明生成成功".to_string(),
                format!("✓ 处理时间: {:?}", processing_time),
            ],
            timestamp,
            processing_time_ms: processing_time.as_millis() as u64,
        };
        
        log::info!("✅ 身份证明生成成功");
        log::info!("   处理时间: {:?}", processing_time);
        
        Ok(result)
    }
    
    /// 验证身份
    pub async fn verify_identity(&self, cid: &str, proof: &crate::ProofResult) -> Result<AuthResult> {
        log::info!("🔍 验证身份");
        
        let start_time = Instant::now();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // 创建nonce
        let nonce = format!("verify_{}", timestamp).into_bytes();
        
        // 验证证明
        let verification = self.identity_manager.verify_identity_with_zkp(
            cid,
            &proof.proof,
            &nonce
        ).await?;
        
        let processing_time = start_time.elapsed();
        
        let result = AuthResult {
            success: verification.zkp_verified,
            agent_id: verification.did.clone(),
            proof: Some(proof.clone()),
            verification_details: verification.verification_details,
            timestamp,
            processing_time_ms: processing_time.as_millis() as u64,
        };
        
        log::info!("✅ 身份验证完成");
        log::info!("   验证结果: {}", if result.success { "通过" } else { "失败" });
        log::info!("   处理时间: {:?}", processing_time);
        
        Ok(result)
    }
    
    /// 双向认证
    pub async fn mutual_authentication(&self, 
        _alice_info: &AgentInfo, alice_keypair: &KeyPair, _alice_peer_id: &PeerId, alice_cid: &str,
        _bob_info: &AgentInfo, bob_keypair: &KeyPair, _bob_peer_id: &PeerId, bob_cid: &str
    ) -> Result<(AuthResult, AuthResult, AuthResult, AuthResult)> {
        log::info!("🔄 开始双向认证流程");
        
        // Alice生成证明
        let alice_proof = self.generate_proof(alice_keypair, alice_cid).await?;
        
        // Bob验证Alice
        let bob_verify_alice = self.verify_identity(alice_cid, alice_proof.proof.as_ref().unwrap()).await?;
        
        // Bob生成证明
        let bob_proof = self.generate_proof(bob_keypair, bob_cid).await?;
        
        // Alice验证Bob
        let alice_verify_bob = self.verify_identity(bob_cid, bob_proof.proof.as_ref().unwrap()).await?;
        
        log::info!("✅ 双向认证完成");
        log::info!("   Alice → Bob: {}", if bob_verify_alice.success { "✅" } else { "❌" });
        log::info!("   Bob → Alice: {}", if alice_verify_bob.success { "✅" } else { "❌" });
        
        Ok((alice_proof, bob_verify_alice, bob_proof, alice_verify_bob))
    }
    
    /// 批量认证测试
    pub async fn batch_authentication_test(&self, 
        _agent_info: &AgentInfo, keypair: &KeyPair, _peer_id: &PeerId, cid: &str, count: usize
    ) -> Result<BatchAuthResult> {
        log::info!("🔄 开始批量认证测试: {}次", count);
        
        let start_time = Instant::now();
        let mut results = Vec::new();
        let mut success_count = 0;
        
        for i in 0..count {
            log::info!("   处理第{}个认证...", i + 1);
            
            // 生成证明
            let proof_result = self.generate_proof(keypair, cid).await?;
            if proof_result.success {
                success_count += 1;
            }
            
            // 验证证明
            let verify_result = self.verify_identity(cid, proof_result.proof.as_ref().unwrap()).await?;
            if verify_result.success {
                success_count += 1;
            }
            
            results.push(proof_result);
            results.push(verify_result);
        }
        
        let total_time = start_time.elapsed();
        let failure_count = (count * 2) - success_count;
        let success_rate = (success_count as f64 / (count * 2) as f64) * 100.0;
        let average_time = total_time.as_millis() as u64 / (count * 2) as u64;
        
        let batch_result = BatchAuthResult {
            total_count: count * 2,
            success_count,
            failure_count,
            success_rate,
            total_time_ms: total_time.as_millis() as u64,
            average_time_ms: average_time,
            results,
        };
        
        log::info!("✅ 批量认证测试完成");
        log::info!("   总处理数: {}", batch_result.total_count);
        log::info!("   成功数: {}", batch_result.success_count);
        log::info!("   成功率: {:.2}%", batch_result.success_rate);
        log::info!("   总时间: {:?}", total_time);
        log::info!("   平均时间: {}ms", batch_result.average_time_ms);
        
        Ok(batch_result)
    }
    
    /// 获取节点信息
    pub async fn get_node_info(&self) -> Result<crate::IpfsNodeInfo> {
        self.ipfs_node_manager.get_node_info().await
    }
    
    /// 获取节点状态
    pub async fn get_node_status(&self) -> crate::IpfsNodeStatus {
        self.ipfs_node_manager.status().await
    }
    
    /// 停止节点
    pub async fn stop(&self) -> Result<()> {
        self.ipfs_node_manager.stop().await
    }
}
