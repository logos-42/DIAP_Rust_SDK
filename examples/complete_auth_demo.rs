use diap_rs_sdk::{
    AgentAuthManager, AuthResult
};
use std::time::Instant;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 完整智能体认证闭环演示");
    println!("==========================================");
    
    // 初始化认证管理器
    println!("\n🔧 初始化智能体认证管理器...");
    let start_time = Instant::now();
    let auth_manager = AgentAuthManager::new().await?;
    let init_time = start_time.elapsed();
    
    println!("✅ 认证管理器初始化成功");
    println!("   初始化时间: {:?}", init_time);
    println!("   节点状态: {:?}", auth_manager.get_node_status().await);
    
    // 获取节点信息
    match auth_manager.get_node_info().await {
        Ok(info) => {
            println!("   节点ID: {}", info.id);
            println!("   版本: {}", info.agent_version);
        }
        Err(e) => {
            println!("   ⚠️  节点信息获取失败: {}", e);
        }
    }
    
    println!("\n🤖 创建智能体A (Alice)");
    println!("==========================");
    
    // 创建Alice
    let (alice_info, alice_keypair, alice_peer_id) = auth_manager.create_agent("Alice", Some("alice@example.com"))?;
    
    // 注册Alice身份
    println!("\n📝 注册Alice身份...");
    let alice_registration = auth_manager.register_agent(&alice_info, &alice_keypair, &alice_peer_id).await?;
    println!("✅ Alice身份注册成功");
    println!("   CID: {}", alice_registration.cid);
    
    println!("\n🤖 创建智能体B (Bob)");
    println!("========================");
    
    // 创建Bob
    let (bob_info, bob_keypair, bob_peer_id) = auth_manager.create_agent("Bob", Some("bob@example.com"))?;
    
    // 注册Bob身份
    println!("\n📝 注册Bob身份...");
    let bob_registration = auth_manager.register_agent(&bob_info, &bob_keypair, &bob_peer_id).await?;
    println!("✅ Bob身份注册成功");
    println!("   CID: {}", bob_registration.cid);
    
    println!("\n🔄 智能体间认证流程");
    println!("====================");
    
    // 双向认证
    println!("\n🤝 开始双向认证...");
    let mutual_start = Instant::now();
    
    let (alice_proof, bob_verify_alice, bob_proof, alice_verify_bob) = auth_manager.mutual_authentication(
        &alice_info, &alice_keypair, &alice_peer_id, &alice_registration.cid,
        &bob_info, &bob_keypair, &bob_peer_id, &bob_registration.cid
    ).await?;
    
    let mutual_time = mutual_start.elapsed();
    
    println!("✅ 双向认证完成");
    println!("   总时间: {:?}", mutual_time);
    println!("   Alice → Bob: {}", if bob_verify_alice.success { "✅ 通过" } else { "❌ 失败" });
    println!("   Bob → Alice: {}", if alice_verify_bob.success { "✅ 通过" } else { "❌ 失败" });
    
    // 详细认证结果
    println!("\n📊 认证结果详情");
    println!("==================");
    
    println!("Alice证明生成:");
    print_auth_result(&alice_proof);
    
    println!("\nBob验证Alice:");
    print_auth_result(&bob_verify_alice);
    
    println!("\nBob证明生成:");
    print_auth_result(&bob_proof);
    
    println!("\nAlice验证Bob:");
    print_auth_result(&alice_verify_bob);
    
    println!("\n🔄 批量认证测试");
    println!("================");
    
    // 批量认证测试
    let batch_result = auth_manager.batch_authentication_test(
        &alice_info, &alice_keypair, &alice_peer_id, &alice_registration.cid, 5
    ).await?;
    
    println!("✅ 批量认证测试完成");
    println!("   总处理数: {}", batch_result.total_count);
    println!("   成功数: {}", batch_result.success_count);
    println!("   失败数: {}", batch_result.failure_count);
    println!("   成功率: {:.2}%", batch_result.success_rate);
    println!("   总时间: {}ms", batch_result.total_time_ms);
    println!("   平均时间: {}ms", batch_result.average_time_ms);
    
    println!("\n📈 性能分析");
    println!("=============");
    
    // 性能分析
    let mut total_proof_time = 0u64;
    let mut total_verify_time = 0u64;
    let mut proof_count = 0;
    let mut verify_count = 0;
    
    for result in &batch_result.results {
        if result.proof.is_some() && result.agent_id.contains("Alice") {
            total_proof_time += result.processing_time_ms;
            proof_count += 1;
        } else if result.agent_id.contains("Alice") {
            total_verify_time += result.processing_time_ms;
            verify_count += 1;
        }
    }
    
    if proof_count > 0 {
        println!("   证明生成平均时间: {}ms", total_proof_time / proof_count as u64);
    }
    if verify_count > 0 {
        println!("   身份验证平均时间: {}ms", total_verify_time / verify_count as u64);
    }
    
    println!("\n🎯 认证闭环验证");
    println!("================");
    
    // 验证认证闭环的完整性
    let alice_authenticated = bob_verify_alice.success;
    let bob_authenticated = alice_verify_bob.success;
    let mutual_trust = alice_authenticated && bob_authenticated;
    
    println!("   Alice身份验证: {}", if alice_authenticated { "✅ 成功" } else { "❌ 失败" });
    println!("   Bob身份验证: {}", if bob_authenticated { "✅ 成功" } else { "❌ 失败" });
    println!("   相互信任建立: {}", if mutual_trust { "✅ 成功" } else { "❌ 失败" });
    
    if mutual_trust {
        println!("\n🎉 智能体认证闭环建立成功！");
        println!("   两个智能体现在可以安全地进行通信");
        println!("   身份验证通过ZKP证明，无需信任第三方");
        println!("   所有操作都在去中心化环境中完成");
    } else {
        println!("\n⚠️  认证闭环建立失败");
        println!("   请检查ZKP证明和验证过程");
    }
    
    println!("\n🔧 系统状态");
    println!("============");
    println!("   节点状态: {:?}", auth_manager.get_node_status().await);
    println!("   认证管理器: 运行中");
    println!("   缓存系统: 激活");
    
    println!("\n💡 下一步可以：");
    println!("   1. 实现智能体间消息传递");
    println!("   2. 添加更多智能体到网络");
    println!("   3. 实现分布式共识机制");
    println!("   4. 添加监控和日志系统");
    
    // 清理资源
    println!("\n🧹 清理资源...");
    auth_manager.stop().await?;
    println!("✅ 资源清理完成");
    
    println!("\n🎊 完整认证闭环演示完成！");
    println!("==========================================");
    
    Ok(())
}

/// 打印认证结果详情
fn print_auth_result(result: &AuthResult) {
    println!("   智能体ID: {}", result.agent_id);
    println!("   成功状态: {}", if result.success { "✅" } else { "❌" });
    println!("   处理时间: {}ms", result.processing_time_ms);
    println!("   时间戳: {}", result.timestamp);
    
    if !result.verification_details.is_empty() {
        println!("   验证详情:");
        for detail in &result.verification_details {
            println!("     {}", detail);
        }
    }
    
    if let Some(proof) = &result.proof {
        println!("   证明长度: {} 字节", proof.proof.len());
    }
}
