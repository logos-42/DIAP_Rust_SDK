// DIAP Rust SDK - Noir ZKP认证演示
// 展示使用Noir进行智能体认证的优势

use diap_rs_sdk::{
    NoirAgent, 
    NoirProofResult, 
    ImprovedNoirZKPManager,
    KeyPair, 
    AgentInfo
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    env_logger::init();
    
    println!("🚀 Noir ZKP智能体认证演示");
    println!("==========================================");
    
    // 1. 创建智能体
    println!("\n🤖 创建智能体...");
    let agent_info = AgentInfo {
        name: "Noir Agent".to_string(),
        services: vec![],
        description: Some("Noir Agent for ZKP authentication".to_string()),
        tags: Some(vec!["noir".to_string(), "zkp".to_string()]),
    };
    
    let mut agent = NoirAgent::new(
        "./noir_circuits".to_string(),
        agent_info,
    )?;
    
    println!("✅ 智能体创建成功");
    println!("   DID: {}", agent.get_did());
    
    // 2. 生成访问证明
    println!("\n🔐 生成访问证明...");
    let resource_cid = "QmTestResource123456789";
    let challenge_nonce = b"challenge_123";
    
    let start_time = std::time::Instant::now();
    let proof_result = agent.prove_access(resource_cid, challenge_nonce).await?;
    let generation_time = start_time.elapsed();
    
    println!("✅ 证明生成成功");
    println!("   证明大小: {} 字节", proof_result.proof.len());
    println!("   公共输入大小: {} 字节", proof_result.public_inputs.len());
    println!("   生成时间: {:?}", generation_time);
    println!("   电路输出: {}", proof_result.circuit_output);
    
    // 3. 验证证明
    println!("\n🔍 验证证明...");
    let verifier = ImprovedNoirZKPManager::new("./noir_circuits".to_string());
    
    let start_time = std::time::Instant::now();
    let verification_result = verifier.verify_proof(
        &proof_result.proof,
        &proof_result.public_inputs,
        &proof_result.circuit_output,
    ).await?;
    let verification_time = start_time.elapsed();
    
    println!("✅ 验证完成");
    println!("   验证结果: {}", if verification_result.is_valid { "通过" } else { "失败" });
    println!("   验证时间: {:?}", verification_time);
    
    if let Some(error) = verification_result.error_message {
        println!("   错误信息: {}", error);
    }
    
    // 4. 性能对比
    println!("\n📊 性能对比");
    println!("==========================================");
    println!("Noir ZKP方案:");
    println!("   证明生成: {:?}", generation_time);
    println!("   证明验证: {:?}", verification_time);
    println!("   总时间: {:?}", generation_time + verification_time);
    
    let metrics = agent.get_metrics();
    println!("\n📈 性能指标:");
    println!("   总证明生成数: {}", metrics.total_proofs_generated);
    println!("   总证明验证数: {}", metrics.total_proofs_verified);
    println!("   缓存命中率: {:.2}%", metrics.cache_hit_rate);
    
    // 5. 批量测试
    println!("\n🔄 批量测试...");
    let test_count = 5;
    let mut total_generation = std::time::Duration::new(0, 0);
    let mut total_verification = std::time::Duration::new(0, 0);
    let mut success_count = 0;
    
    for i in 0..test_count {
        let test_cid = format!("QmTestResource{}", i);
        let test_nonce = format!("challenge_{}", i).into_bytes();
        
        // 生成证明
        let start = std::time::Instant::now();
        let proof = agent.prove_access(&test_cid, &test_nonce).await?;
        total_generation += start.elapsed();
        
        // 验证证明
        let start = std::time::Instant::now();
        let result = verifier.verify_proof(
            &proof.proof,
            &proof.public_inputs,
            &proof.circuit_output,
        ).await?;
        total_verification += start.elapsed();
        
        if result.is_valid {
            success_count += 1;
        }
        
        println!("   测试 {}: {}", i + 1, if result.is_valid { "✅" } else { "❌" });
    }
    
    println!("\n📊 批量测试结果:");
    println!("   成功率: {}/{} ({:.1}%)", 
             success_count, test_count, 
             (success_count as f64 / test_count as f64) * 100.0);
    println!("   平均生成时间: {:?}", total_generation / test_count);
    println!("   平均验证时间: {:?}", total_verification / test_count);
    println!("   平均总时间: {:?}", (total_generation + total_verification) / test_count);
    
    // 6. 清理缓存
    println!("\n🧹 清理缓存...");
    agent.clear_cache();
    println!("✅ 缓存清理完成");
    
    println!("\n🎉 Noir ZKP认证演示完成！");
    println!("==========================================");
    
    Ok(())
}
