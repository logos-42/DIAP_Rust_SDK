use crate::{AgentInfo, IdentityManager, IdentityRegistration, KeyPair, ServiceInfo};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid;

/// 智能体认证管理器 - 统一的API接口（轻量级版本）
pub struct AgentAuthManager {
    identity_manager: IdentityManager,
}

/// 认证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub agent_id: String,
    pub proof: Option<Vec<u8>>,
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
    /// 创建新的智能体认证管理器（轻量级版本）
    pub async fn new() -> Result<Self> {
        log::info!("🚀 初始化智能体认证管理器（轻量级版本）");

        // 创建轻量级IPFS客户端（仅使用公共网关）
        let ipfs_client = crate::IpfsClient::new_public_only(30);

        // 确保密钥文件存在
        let pk_path = "zkp_proving.key";
        let vk_path = "zkp_verifying.key";

        // 直接使用arkworks-rs生成密钥
        crate::key_generator::ensure_zkp_keys_exist(pk_path, vk_path)?;

        let identity_manager = IdentityManager::new_with_keys(ipfs_client, pk_path, vk_path)?;

        Ok(Self { identity_manager })
    }

    /// 创建带远程IPFS节点的智能体认证管理器
    pub async fn new_with_remote_ipfs(api_url: String, gateway_url: String) -> Result<Self> {
        log::info!("🚀 初始化智能体认证管理器（使用远程IPFS）");

        // 创建带远程节点的IPFS客户端
        let ipfs_client = crate::IpfsClient::new_with_remote_node(api_url, gateway_url, 30);

        // 确保密钥文件存在
        let pk_path = "zkp_proving.key";
        let vk_path = "zkp_verifying.key";

        // 直接使用arkworks-rs生成密钥
        crate::key_generator::ensure_zkp_keys_exist(pk_path, vk_path)?;

        let identity_manager = IdentityManager::new_with_keys(ipfs_client, pk_path, vk_path)?;

        Ok(Self { identity_manager })
    }

    /// 创建智能体
    pub fn create_agent(
        &self,
        name: &str,
        _email: Option<&str>,
    ) -> Result<(AgentInfo, KeyPair, String)> {
        log::info!("🤖 创建智能体: {}", name);

        let agent_info = AgentInfo {
            name: name.to_string(),
            services: vec![ServiceInfo {
                service_type: "API".to_string(),
                endpoint: serde_json::json!("https://api.example.com"),
            }],
            description: Some(format!("{} - DIAP智能体", name)),
            tags: Some(vec!["diap".to_string(), "agent".to_string()]),
        };

        let keypair = KeyPair::generate()?;
        let peer_id = format!("12D3KooW{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..32].to_uppercase());

        log::info!("✅ 智能体创建成功: {}", name);
        log::info!("   DID: {}", keypair.did);

        Ok((agent_info, keypair, peer_id))
    }

    /// 注册智能体身份
    pub async fn register_agent(
        &self,
        agent_info: &AgentInfo,
        keypair: &KeyPair,
        peer_id: &str,
    ) -> Result<IdentityRegistration> {
        log::info!("📝 注册智能体身份: {}", agent_info.name);

        let start_time = Instant::now();
        let registration = self
            .identity_manager
            .register_identity(agent_info, keypair, peer_id)
            .await?;
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
        let did_document =
            crate::get_did_document_from_cid(&self.identity_manager.ipfs_client(), cid).await?;

        // 生成证明
        let proof =
            self.identity_manager
                .generate_binding_proof(keypair, &did_document, cid, &nonce)?;

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
    pub async fn verify_identity(&self, cid: &str, proof: &Vec<u8>) -> Result<AuthResult> {
        log::info!("🔍 验证身份");

        let start_time = Instant::now();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // 创建nonce
        let nonce = format!("verify_{}", timestamp).into_bytes();

        // 验证证明
        let verification = self
            .identity_manager
            .verify_identity_with_zkp(cid, proof, &nonce)
            .await?;

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
        log::info!(
            "   验证结果: {}",
            if result.success { "通过" } else { "失败" }
        );
        log::info!("   处理时间: {:?}", processing_time);

        Ok(result)
    }

    /// 双向认证流程（完整版）
    pub async fn bidirectional_auth(
        &self,
        _alice_info: &AgentInfo,
        alice_keypair: &KeyPair,
        _alice_peer_id: &str,
        alice_cid: &str,
        _bob_info: &AgentInfo,
        bob_keypair: &KeyPair,
        _bob_peer_id: &str,
        bob_cid: &str,
    ) -> Result<(AuthResult, AuthResult, AuthResult)> {
        log::info!("🔄 开始双向认证流程");

        // Alice生成证明
        let alice_proof = self.generate_proof(alice_keypair, alice_cid).await?;

        // Bob验证Alice
        let bob_verify_alice = self
            .verify_identity(alice_cid, alice_proof.proof.as_ref().unwrap())
            .await?;

        // Bob生成证明
        let bob_proof = self.generate_proof(bob_keypair, bob_cid).await?;

        // Alice验证Bob
        let alice_verify_bob = self
            .verify_identity(bob_cid, bob_proof.proof.as_ref().unwrap())
            .await?;

        log::info!("✅ 双向认证完成");
        log::info!(
            "   Alice → Bob: {}",
            if bob_verify_alice.success {
                "✅"
            } else {
                "❌"
            }
        );
        log::info!(
            "   Bob → Alice: {}",
            if alice_verify_bob.success {
                "✅"
            } else {
                "❌"
            }
        );

        Ok((alice_proof, bob_verify_alice, alice_verify_bob))
    }

    /// 批量认证测试
    pub async fn batch_auth_test(
        &self,
        _agent_info: &AgentInfo,
        keypair: &KeyPair,
        _peer_id: &str,
        cid: &str,
        count: usize,
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
            let verify_result = self
                .verify_identity(cid, proof_result.proof.as_ref().unwrap())
                .await?;
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
}
