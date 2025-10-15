// DIAP Rust SDK - 智能体验证闭环演示
// 展示完整的智能体验证流程

use diap_rs_sdk::{
    AgentVerificationManager,
    AgentVerificationRequest,
    KeyPair,
    DIDDocument,
};
use anyhow::Result;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    env_logger::init();
    
    println!("🚀 智能体验证闭环演示");
    println!("==========================================");
    
    // 1. 创建验证管理器
    println!("\n🔧 初始化验证管理器...");
    let mut verification_manager = AgentVerificationManager::new("./noir_circuits".to_string());
    println!("✅ 验证管理器初始化完成");
    
    // 2. 创建测试智能体
    println!("\n🤖 创建测试智能体...");
    let keypair = KeyPair::generate()?;
    let did_document = DIDDocument {
        context: vec!["https://www.w3.org/ns/did/v1".to_string()],
        id: keypair.did.clone(),
        verification_method: vec![],
        authentication: vec![],
        service: None,
        created: chrono::Utc::now().to_rfc3339(),
    };
    
    println!("✅ 智能体创建完成");
    println!("   DID: {}", keypair.did);
    println!("   公钥: {}...", hex::encode(&keypair.public_key[..8]));
    
    // 3. 创建验证请求
    println!("\n📝 创建验证请求...");
    let request = AgentVerificationRequest {
        agent_id: "agent_001".to_string(),
        resource_cid: "QmTestResource123456789".to_string(),
        challenge_nonce: "challenge_nonce_123".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
        expiry_seconds: 3600, // 1小时过期
    };
    
    println!("✅ 验证请求创建完成");
    println!("   智能体ID: {}", request.agent_id);
    println!("   资源CID: {}", request.resource_cid);
    println!("   挑战nonce: {}", request.challenge_nonce);
    
    // 4. 执行智能体验证
    println!("\n🔍 执行智能体验证...");
    let did_doc_json = serde_json::to_string(&did_document)?;
    
    let verification_response = verification_manager.verify_agent_access(
        &request,
        &keypair.private_key,
        &did_doc_json,
    ).await?;
    
    match verification_response.status {
        diap_rs_sdk::AgentVerificationStatus::Verified => {
            println!("✅ 智能体验证成功！");
            println!("   证明大小: {} bytes", 
                verification_response.proof.as_ref().map_or(0, |p| p.len()));
            println!("   电路输出: {}", 
                verification_response.circuit_output.as_ref().unwrap_or(&"N/A".to_string()));
        }
        diap_rs_sdk::AgentVerificationStatus::Failed => {
            println!("❌ 智能体验证失败");
            if let Some(error) = verification_response.error_message {
                println!("   错误: {}", error);
            }
        }
        diap_rs_sdk::AgentVerificationStatus::Expired => {
            println!("⏰ 验证请求已过期");
        }
        _ => {
            println!("⚠️  验证状态: {:?}", verification_response.status);
        }
    }
    
    // 5. 验证证明
    if let (Some(proof), Some(public_inputs), Some(circuit_output)) = 
        (&verification_response.proof, &verification_response.public_inputs, &verification_response.circuit_output) {
        
        println!("\n🔍 验证生成的证明...");
        match verification_manager.verify_agent_proof(proof, public_inputs, circuit_output).await {
            Ok(is_valid) => {
                println!("✅ 证明验证: {}", if is_valid { "成功" } else { "失败" });
            }
            Err(e) => {
                println!("❌ 证明验证出错: {}", e);
            }
        }
    }
    
    // 6. 批量验证演示
    println!("\n🔄 批量验证演示...");
    
    // 创建多个智能体
    let mut agent_data = HashMap::new();
    for i in 1..=3 {
        let agent_keypair = KeyPair::generate()?;
        let agent_did_doc = DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: agent_keypair.did.clone(),
            verification_method: vec![],
            authentication: vec![],
            service: None,
            created: chrono::Utc::now().to_rfc3339(),
        };
        
        agent_data.insert(
            format!("agent_{:03}", i),
            (agent_keypair.private_key.to_vec(), serde_json::to_string(&agent_did_doc)?)
        );
    }
    
    // 创建批量验证请求
    let mut batch_requests = Vec::new();
    for i in 1..=3 {
        batch_requests.push(AgentVerificationRequest {
            agent_id: format!("agent_{:03}", i),
            resource_cid: format!("QmResource{}", i),
            challenge_nonce: format!("challenge_{}", i),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            expiry_seconds: 3600,
        });
    }
    
    // 执行批量验证
    let batch_responses = verification_manager.batch_verify_agents(batch_requests, agent_data).await?;
    
    println!("✅ 批量验证完成");
    let success_count = batch_responses.iter()
        .filter(|r| matches!(r.status, diap_rs_sdk::AgentVerificationStatus::Verified))
        .count();
    println!("   成功率: {}/{}", success_count, batch_responses.len());
    
    // 7. 缓存统计
    println!("\n📊 缓存统计...");
    let cache_stats = verification_manager.get_cache_stats();
    println!("   总缓存条目: {}", cache_stats.total_entries);
    println!("   验证成功: {}", cache_stats.verified_count);
    println!("   验证失败: {}", cache_stats.failed_count);
    println!("   成功率: {:.1}%", cache_stats.success_rate * 100.0);
    
    // 8. 清理过期缓存
    println!("\n🧹 清理过期缓存...");
    verification_manager.cleanup_expired_cache();
    println!("✅ 缓存清理完成");
    
    println!("\n🎉 智能体验证闭环演示完成！");
    println!("==========================================");
    
    println!("\n💡 验证闭环包含:");
    println!("   ✅ 智能体身份验证");
    println!("   ✅ 资源访问权限验证");
    println!("   ✅ ZKP证明生成");
    println!("   ✅ ZKP证明验证");
    println!("   ✅ 缓存机制");
    println!("   ✅ 批量处理");
    println!("   ✅ 过期管理");
    
    Ok(())
}
