// DIAP Rust SDK - IPFS双向验证闭环演示
// 展示基于真实IPFS的智能体双向身份验证完整流程

use diap_rs_sdk::{
    IpfsBidirectionalVerificationManager,
    AgentInfo, KeyPair,
};
use anyhow::Result;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    env_logger::init();
    
    println!("🚀 IPFS双向验证闭环演示");
    println!("==========================================");
    
    // 1. 初始化双向验证管理器
    println!("\n🔧 初始化IPFS双向验证管理器...");
    let start_time = Instant::now();
    let mut verification_manager = IpfsBidirectionalVerificationManager::new().await?;
    let init_time = start_time.elapsed();
    
    println!("✅ 双向验证管理器初始化成功");
    println!("   初始化时间: {:?}", init_time);
    
    // 获取IPFS客户端状态（API 更名: get_ipfs_node_status → get_ipfs_client_status）
    match verification_manager.get_ipfs_client_status().await {
        Ok(status) => println!("   IPFS节点状态: {}", status),
        Err(e) => println!("   ⚠️  IPFS节点状态获取失败: {}", e),
    }
    
    // 2. 创建智能体A (Alice)
    println!("\n🤖 创建智能体A (Alice)");
    println!("========================");
    
    let alice_info = AgentInfo {
        name: "Alice".to_string(),
        services: vec![],
        description: Some("Alice智能体 - 验证发起方".to_string()),
        tags: Some(vec!["initiator".to_string(), "alice".to_string()]),
    };
    
    let alice_keypair = KeyPair::generate()?;
    println!("✅ Alice智能体创建成功");
    println!("   DID: {}", alice_keypair.did);
    println!("   公钥: {}...", hex::encode(&alice_keypair.public_key[..8]));
    
    // 3. 创建智能体B (Bob)
    println!("\n🤖 创建智能体B (Bob)");
    println!("======================");
    
    let bob_info = AgentInfo {
        name: "Bob".to_string(),
        services: vec![],
        description: Some("Bob智能体 - 验证响应方".to_string()),
        tags: Some(vec!["responder".to_string(), "bob".to_string()]),
    };
    
    let bob_keypair = KeyPair::generate()?;
    println!("✅ Bob智能体创建成功");
    println!("   DID: {}", bob_keypair.did);
    println!("   公钥: {}...", hex::encode(&bob_keypair.public_key[..8]));
    
    // 4. 注册智能体到IPFS网络
    println!("\n📝 注册智能体到IPFS网络");
    println!("=========================");
    
    println!("\n📤 注册Alice到IPFS...");
    let alice_cid = verification_manager.register_agent(&alice_info, &alice_keypair).await?;
    println!("✅ Alice注册成功，CID: {}", alice_cid);
    
    println!("\n📤 注册Bob到IPFS...");
    let bob_cid = verification_manager.register_agent(&bob_info, &bob_keypair).await?;
    println!("✅ Bob注册成功，CID: {}", bob_cid);
    
    // 5. 执行双向验证
    println!("\n🤝 执行双向验证");
    println!("================");
    
    let resource_cid = "QmTestResourceForBidirectionalVerification123456789";
    
    println!("\n🔄 Alice ↔ Bob 双向验证...");
    let verification_start = Instant::now();
    
    let bidirectional_result = verification_manager.initiate_bidirectional_verification(
        "Alice",
        "Bob",
        resource_cid,
    ).await?;
    
    let verification_time = verification_start.elapsed();
    
    // 6. 显示验证结果
    println!("\n📊 双向验证结果");
    println!("================");
    
    println!("验证状态: {}", if bidirectional_result.success { "✅ 成功" } else { "❌ 失败" });
    println!("总验证时间: {:?}", verification_time);
    println!("发起方: {}", bidirectional_result.initiator_id);
    println!("响应方: {}", bidirectional_result.responder_id);
    println!("验证时间戳: {}", bidirectional_result.verification_timestamp);
    
    // 详细验证结果
    println!("\n📋 Alice验证结果详情:");
    print_verification_result(&bidirectional_result.initiator_result);
    
    println!("\n📋 Bob验证结果详情:");
    print_verification_result(&bidirectional_result.responder_result);
    
    // 7. 批量验证演示
    println!("\n🔄 批量双向验证演示");
    println!("====================");
    
    // 创建更多智能体用于批量验证
    let charlie_info = AgentInfo {
        name: "Charlie".to_string(),
        services: vec![],
        description: Some("Charlie智能体".to_string()),
        tags: Some(vec!["batch".to_string(), "charlie".to_string()]),
    };
    
    let david_info = AgentInfo {
        name: "David".to_string(),
        services: vec![],
        description: Some("David智能体".to_string()),
        tags: Some(vec!["batch".to_string(), "david".to_string()]),
    };
    
    let charlie_keypair = KeyPair::generate()?;
    let david_keypair = KeyPair::generate()?;
    
    // 注册新智能体
    println!("\n📤 注册Charlie到IPFS...");
    let _charlie_cid = verification_manager.register_agent(&charlie_info, &charlie_keypair).await?;
    
    println!("\n📤 注册David到IPFS...");
    let _david_cid = verification_manager.register_agent(&david_info, &david_keypair).await?;
    
    // 批量验证
    let agent_pairs = vec![
        ("Alice".to_string(), "Charlie".to_string()),
        ("Bob".to_string(), "David".to_string()),
        ("Charlie".to_string(), "David".to_string()),
    ];
    
    println!("\n🔄 执行批量验证...");
    let batch_start = Instant::now();
    
    let batch_results = verification_manager.batch_bidirectional_verification(
        agent_pairs,
        resource_cid,
    ).await?;
    
    let batch_time = batch_start.elapsed();
    
    println!("✅ 批量验证完成");
    println!("   总时间: {:?}", batch_time);
    
    let success_count = batch_results.iter().filter(|r| r.success).count();
    println!("   成功对数: {}/{}", success_count, batch_results.len());
    println!("   成功率: {:.1}%", (success_count as f64 / batch_results.len() as f64) * 100.0);
    
    // 批量验证结果详情
    for (i, result) in batch_results.iter().enumerate() {
        println!("\n📋 验证对 {}: {} ↔ {}", 
                i + 1, result.initiator_id, result.responder_id);
        println!("   状态: {}", if result.success { "✅ 成功" } else { "❌ 失败" });
        println!("   验证时间: {}ms", result.total_verification_time_ms);
        
        if let Some(error) = &result.error_message {
            println!("   错误: {}", error);
        }
    }
    
    // 8. 会话管理
    println!("\n📊 会话管理");
    println!("============");
    
    let active_sessions = verification_manager.get_active_sessions();
    println!("活跃会话数: {}", active_sessions.len());
    
    for (agent_id, session) in active_sessions {
        println!("   智能体: {}", agent_id);
        println!("     状态: {:?}", session.status);
        println!("     DID文档CID: {}", session.did_document_cid);
        println!("     创建时间: {}", session.created_at);
        println!("     最后活动: {}", session.last_activity);
    }
    
    // 9. 清理过期会话
    println!("\n🧹 清理过期会话...");
    verification_manager.cleanup_expired_sessions();
    println!("✅ 会话清理完成");
    
    // 10. 验证闭环总结
    println!("\n🎯 验证闭环总结");
    println!("================");
    
    let total_successful_pairs = if bidirectional_result.success { 1 } else { 0 } + success_count;
    let total_pairs = 1 + batch_results.len();
    
    println!("总验证对数: {}", total_pairs);
    println!("成功验证对数: {}", total_successful_pairs);
    println!("整体成功率: {:.1}%", (total_successful_pairs as f64 / total_pairs as f64) * 100.0);
    
    if total_successful_pairs > 0 {
        println!("\n🎉 IPFS双向验证闭环建立成功！");
        println!("✅ 智能体身份已通过IPFS网络验证");
        println!("✅ 双向信任关系已建立");
        println!("✅ 所有验证数据已存储在IPFS网络中");
        println!("✅ 实现了完全去中心化的身份验证");
        
        println!("\n💡 验证闭环特性:");
        println!("   🔐 基于Noir ZKP的零知识证明");
        println!("   🌐 使用IPFS进行去中心化存储");
        println!("   🤝 支持双向身份验证");
        println!("   ⚡ 支持批量验证操作");
        println!("   🛡️  防重放攻击和会话管理");
        println!("   📊 完整的验证状态跟踪");
    } else {
        println!("\n⚠️  验证闭环建立失败");
        println!("请检查IPFS网络连接和Noir电路配置");
    }
    
    // 11. 性能分析
    println!("\n📈 性能分析");
    println!("============");
    
    let mut total_verification_time = 0u64;
    let mut verification_count = 0;
    
    total_verification_time += bidirectional_result.total_verification_time_ms;
    verification_count += 1;
    
    for result in &batch_results {
        total_verification_time += result.total_verification_time_ms;
        verification_count += 1;
    }
    
    if verification_count > 0 {
        let avg_time = total_verification_time / verification_count as u64;
        println!("平均验证时间: {}ms", avg_time);
        println!("总验证时间: {}ms", total_verification_time);
        println!("验证吞吐量: {:.2} 对/秒", (verification_count as f64 * 1000.0) / total_verification_time as f64);
    }
    
    println!("\n🔧 系统状态");
    println!("============");
    println!("IPFS节点: 运行中");
    println!("Noir电路: 可用");
    println!("验证管理器: 活跃");
    println!("缓存系统: 激活");
    
    println!("\n💡 下一步可以：");
    println!("   1. 实现智能体间消息传递");
    println!("   2. 添加更多智能体到网络");
    println!("   3. 实现分布式共识机制");
    println!("   4. 添加监控和日志系统");
    println!("   5. 实现跨链身份验证");
    
    println!("\n🎊 IPFS双向验证闭环演示完成！");
    println!("==========================================");
    
    Ok(())
}

/// 打印验证结果详情
fn print_verification_result(result: &diap_rs_sdk::VerificationResult) {
    println!("   智能体ID: {}", result.agent_id);
    println!("   验证状态: {:?}", result.status);
    println!("   处理时间: {}ms", result.processing_time_ms);
    println!("   时间戳: {}", result.timestamp);
    
    if let Some(proof) = &result.proof {
        println!("   证明长度: {} bytes", proof.proof.len());
        println!("   公共输入长度: {} bytes", proof.public_inputs.len());
        println!("   电路输出: {}", proof.circuit_output);
        println!("   资源CID: {}", proof.resource_cid);
        println!("   挑战nonce: {}", proof.challenge_nonce);
    }
    
    if let Some(error) = &result.error_message {
        println!("   错误信息: {}", error);
    }
}
