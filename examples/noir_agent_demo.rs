// DIAP Rust SDK - Noir Agent Demo
// 展示使用Noir ZKP的开发者友好API

use diap_rs_sdk::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("\n🚀 DIAP Noir Agent Demo - 开发者友好API\n");
    println!("========================================");
    println!("新特性：");
    println!("  ✓ 使用Noir电路（替代arkworks-rs）");
    println!("  ✓ 开发者友好的API设计");
    println!("  ✓ 智能性能优化和缓存");
    println!("  ✓ 完全去中心化（无第三方依赖）");
    println!("========================================\n");
    
    // ==================== 第1步：创建智能体 ====================
    println!("🤖 第1步：创建智能体");
    
    let agent_info = AgentInfo {
        name: "我的Noir智能体".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "API".to_string(),
                endpoint: serde_json::json!("https://api.example.com"),
            },
            ServiceInfo {
                service_type: "Chat".to_string(),
                endpoint: serde_json::json!("https://chat.example.com"),
            },
        ],
        description: Some("基于Noir ZKP的高性能智能体".to_string()),
        tags: Some(vec!["noir".to_string(), "zkp".to_string(), "fast".to_string()]),
    };
    
    let mut agent = NoirAgent::new(
        "./noir_circuits".to_string(),
        agent_info,
    )?;
    
    println!("✓ 智能体创建成功");
    println!("  DID: {}", agent.get_did());
    println!();
    
    // ==================== 第2步：生成访问证明 ====================
    println!("🔐 第2步：生成访问证明");
    
    let resource_cid = "QmTestResourceCID123456789";
    let challenge_nonce = b"challenge_from_resource_node";
    
    println!("  请求访问资源: {}", resource_cid);
    println!("  挑战nonce: {}", hex::encode(challenge_nonce));
    
    // 第一次生成证明（冷启动）
    let start_time = std::time::Instant::now();
    let proof1 = agent.prove_access(resource_cid, challenge_nonce).await?;
    let first_generation_time = start_time.elapsed();
    
    println!("✓ 第一次证明生成完成");
    println!("  生成时间: {}ms", first_generation_time.as_millis());
    println!("  电路输出: {}", proof1.circuit_output);
    println!();
    
    // 第二次生成证明（应该使用缓存）
    let start_time = std::time::Instant::now();
    let _proof2 = agent.prove_access(resource_cid, challenge_nonce).await?;
    let second_generation_time = start_time.elapsed();
    
    println!("✓ 第二次证明生成完成（使用缓存）");
    println!("  生成时间: {}ms", second_generation_time.as_millis());
    println!("  性能提升: {}x", first_generation_time.as_millis() as f64 / second_generation_time.as_millis() as f64);
    println!();
    
    // ==================== 第3步：验证证明 ====================
    println!("🔍 第3步：验证证明");
    
    let mut zkp_manager = NoirZKPManager::new("./noir_circuits".to_string());
    let is_valid = zkp_manager.verify_did_binding_proof(
        &proof1.proof,
        &proof1.public_inputs,
        &proof1.circuit_output,
    ).await?;
    
    println!("✓ 证明验证完成");
    println!("  验证结果: {}", if is_valid { "✅ 有效" } else { "❌ 无效" });
    println!();
    
    // ==================== 第4步：性能统计 ====================
    println!("📊 第4步：性能统计");
    
    let metrics = agent.get_metrics();
    println!("✓ 性能指标:");
    println!("  总证明生成次数: {}", metrics.total_proofs_generated);
    println!("  总证明验证次数: {}", metrics.total_proofs_verified);
    println!("  平均生成时间: {}ms", metrics.proof_generation_time_ms);
    println!("  平均验证时间: {}ms", metrics.proof_verification_time_ms);
    println!("  缓存命中率: {:.2}%", metrics.cache_hit_rate * 100.0);
    println!();
    
    // ==================== 第5步：批量证明生成 ====================
    println!("🔄 第5步：批量证明生成测试");
    
    let resources = vec![
        "QmResource1",
        "QmResource2", 
        "QmResource3",
    ];
    
    let total_time = std::time::Instant::now();
    
    for (i, resource) in resources.iter().enumerate() {
        let challenge = format!("challenge_{}", i).into_bytes();
        let start_time = std::time::Instant::now();
        
        let _proof = agent.prove_access(resource, &challenge).await?;
        
        let generation_time = start_time.elapsed();
        println!("  ✓ 资源 {}: {}ms", i + 1, generation_time.as_millis());
    }
    
    let batch_total_time = total_time.elapsed();
    println!("✓ 批量证明生成完成");
    println!("  总时间: {}ms", batch_total_time.as_millis());
    println!("  平均每个: {}ms", batch_total_time.as_millis() / resources.len() as u128);
    println!();
    
    // ==================== 第6步：缓存管理 ====================
    println!("🧹 第6步：缓存管理");
    
    println!("  缓存前性能指标:");
    let metrics_before = agent.get_metrics();
    println!("    总证明生成: {}", metrics_before.total_proofs_generated);
    
    agent.clear_cache();
    
    println!("✓ 缓存已清理");
    println!("  重新生成证明以测试缓存重建...");
    
    let start_time = std::time::Instant::now();
    let _proof = agent.prove_access(resource_cid, challenge_nonce).await?;
    let rebuild_time = start_time.elapsed();
    
    println!("✓ 缓存重建完成");
    println!("  重建时间: {}ms", rebuild_time.as_millis());
    println!();
    
    // ==================== 总结 ====================
    println!("🎉 演示完成！");
    println!("========================================");
    println!("Noir ZKP方案的优势：");
    println!("  ✓ 开发效率：直观的电路描述");
    println!("  ✓ 性能优化：智能缓存和批量处理");
    println!("  ✓ 完全去中心化：无第三方依赖");
    println!("  ✓ 易于维护：清晰的代码结构");
    println!("  ✓ 高性能：优化的证明生成");
    println!("========================================\n");
    
    Ok(())
}

