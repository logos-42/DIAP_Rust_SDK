use diap_rs_sdk::{
    NoirAgent, 
    IpfsClient, 
    IdentityManager, 
    AgentInfo, 
    IpfsNodeManager,
    IpfsNodeConfig,
    KeyPair
};
use libp2p_identity::PeerId;
use std::time::Instant;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🤖 智能体间认证闭环演示");
    println!("================================");
    
    // 启动内置IPFS节点
    println!("\n🚀 启动内置IPFS节点...");
    let ipfs_config = IpfsNodeConfig {
        data_dir: std::env::temp_dir().join("diap_ipfs_test"),
        api_port: 5001,
        gateway_port: 8080,
        auto_start: true,
        startup_timeout: 30,
        enable_bootstrap: true,
        enable_swarm: true,
        swarm_port: 4001,
        verbose_logging: false,
    };
    
    let ipfs_manager = IpfsNodeManager::new(ipfs_config);
    ipfs_manager.start().await?;
    
    // 创建IPFS客户端
    let (ipfs_client, _ipfs_manager) = IpfsClient::new_with_builtin_node(
        Some(ipfs_config), 
        None, 
        None, 
        None, 
        None, 
        30
    ).await?;
    
    // 创建身份管理器
    let identity_manager = IdentityManager::new_with_builtin_ipfs(
        ipfs_client.clone(),
        "http://localhost:5001",
        "http://localhost:8080",
        30
    ).await?;
    
    println!("\n🔐 创建智能体A (Alice)");
    println!("========================");
    
    // 创建智能体A
    let alice_info = AgentInfo {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        services: vec![],
    };
    
    let alice_keypair = KeyPair::generate()?;
    let alice_peer_id = PeerId::random();
    
    println!("✅ Alice创建成功");
    println!("   DID: {}", alice_keypair.did);
    println!("   公钥: {}", alice_keypair.public_key);
    
    // Alice注册身份到IPFS
    println!("\n📝 Alice注册身份到IPFS...");
    let alice_registration = identity_manager.register_identity(&alice_info, &alice_keypair, &alice_peer_id).await?;
    println!("✅ Alice身份注册成功");
    println!("   CID: {}", alice_registration.cid);
    
    // 创建Alice的Noir ZKP代理
    println!("\n🔮 创建Alice的Noir ZKP代理...");
    let alice_noir = NoirAgent::new(&alice_info, &ipfs_client)?;
    println!("✅ Alice Noir代理创建成功");
    
    println!("\n🔐 创建智能体B (Bob)");
    println!("========================");
    
    // 创建智能体B
    let bob_info = AgentInfo {
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        services: vec![],
    };
    
    let bob_keypair = KeyPair::generate()?;
    let bob_peer_id = PeerId::random();
    
    println!("✅ Bob创建成功");
    println!("   DID: {}", bob_keypair.did);
    println!("   公钥: {}", bob_keypair.public_key);
    
    // Bob注册身份到IPFS
    println!("\n📝 Bob注册身份到IPFS...");
    let bob_registration = identity_manager.register_identity(&bob_info, &bob_keypair, &bob_peer_id).await?;
    println!("✅ Bob身份注册成功");
    println!("   CID: {}", bob_registration.cid);
    
    // 创建Bob的Noir ZKP代理
    println!("\n🔮 创建Bob的Noir ZKP代理...");
    let bob_noir = NoirAgent::new(&bob_info, &ipfs_client)?;
    println!("✅ Bob Noir代理创建成功");
    
    println!("\n🔄 智能体间身份验证流程");
    println!("==========================");
    
    // Alice向Bob证明自己的身份
    println!("\n📤 Alice向Bob证明身份...");
    let start_time = Instant::now();
    
    let alice_proof = alice_noir.prove_access(&alice_registration.cid).await?;
    let alice_proof_time = start_time.elapsed();
    
    println!("✅ Alice身份证明生成成功");
    println!("   证明时间: {:?}", alice_proof_time);
    println!("   证明内容: {}", alice_proof);
    
    // Bob验证Alice的身份
    println!("\n🔍 Bob验证Alice的身份...");
    let start_time = Instant::now();
    
    let verification_result = bob_noir.verify_proof(&alice_proof, &alice_keypair.did, &alice_registration.cid).await?;
    let verification_time = start_time.elapsed();
    
    println!("✅ Alice身份验证完成");
    println!("   验证时间: {:?}", verification_time);
    println!("   验证结果: {}", if verification_result { "✅ 通过" } else { "❌ 失败" });
    
    // Bob向Alice证明自己的身份
    println!("\n📤 Bob向Alice证明身份...");
    let start_time = Instant::now();
    
    let bob_proof = bob_noir.prove_access(&bob_registration.cid).await?;
    let bob_proof_time = start_time.elapsed();
    
    println!("✅ Bob身份证明生成成功");
    println!("   证明时间: {:?}", bob_proof_time);
    println!("   证明内容: {}", bob_proof);
    
    // Alice验证Bob的身份
    println!("\n🔍 Alice验证Bob的身份...");
    let start_time = Instant::now();
    
    let verification_result = alice_noir.verify_proof(&bob_proof, &bob_keypair.did, &bob_registration.cid).await?;
    let verification_time = start_time.elapsed();
    
    println!("✅ Bob身份验证完成");
    println!("   验证时间: {:?}", verification_time);
    println!("   验证结果: {}", if verification_result { "✅ 通过" } else { "❌ 失败" });
    
    println!("\n📊 性能统计");
    println!("=============");
    
    // 获取缓存统计
    let alice_stats = alice_noir.get_cache_stats().await?;
    let bob_stats = bob_noir.get_cache_stats().await?;
    
    println!("Alice缓存统计:");
    println!("  缓存命中: {}", alice_stats.cache_hits);
    println!("  缓存未命中: {}", alice_stats.cache_misses);
    println!("  缓存命中率: {:.2}%", 
        if alice_stats.cache_hits + alice_stats.cache_misses > 0 {
            (alice_stats.cache_hits as f64 / (alice_stats.cache_hits + alice_stats.cache_misses) as f64) * 100.0
        } else { 0.0 }
    );
    
    println!("Bob缓存统计:");
    println!("  缓存命中: {}", bob_stats.cache_hits);
    println!("  缓存未命中: {}", bob_stats.cache_misses);
    println!("  缓存命中率: {:.2}%", 
        if bob_stats.cache_hits + bob_stats.cache_misses > 0 {
            (bob_stats.cache_hits as f64 / (bob_stats.cache_hits + bob_stats.cache_misses) as f64) * 100.0
        } else { 0.0 }
    );
    
    println!("\n🔄 批量验证演示");
    println!("================");
    
    // 批量生成和验证证明
    let mut batch_proofs = Vec::new();
    let batch_start = Instant::now();
    
    for i in 0..5 {
        println!("生成第{}个证明...", i + 1);
        let proof = alice_noir.prove_access(&alice_registration.cid).await?;
        batch_proofs.push(proof);
    }
    
    let batch_proof_time = batch_start.elapsed();
    println!("✅ 批量证明生成完成，总时间: {:?}", batch_proof_time);
    println!("   平均每个证明: {:?}", batch_proof_time / 5);
    
    // 批量验证
    let batch_verify_start = Instant::now();
    let mut success_count = 0;
    
    for (i, proof) in batch_proofs.iter().enumerate() {
        let result = bob_noir.verify_proof(proof, &alice_keypair.did, &alice_registration.cid).await?;
        if result {
            success_count += 1;
        }
        println!("验证第{}个证明: {}", i + 1, if result { "✅ 成功" } else { "❌ 失败" });
    }
    
    let batch_verify_time = batch_verify_start.elapsed();
    println!("✅ 批量验证完成，总时间: {:?}", batch_verify_time);
    println!("   平均每个验证: {:?}", batch_verify_time / 5);
    println!("   成功率: {}/5 ({:.1}%)", success_count, (success_count as f64 / 5.0) * 100.0);
    
    println!("\n🎉 智能体间认证闭环演示完成！");
    println!("================================");
    
    // 清理缓存
    println!("\n🧹 清理缓存...");
    alice_noir.clear_cache().await?;
    bob_noir.clear_cache().await?;
    println!("✅ 缓存清理完成");
    
    Ok(())
}
