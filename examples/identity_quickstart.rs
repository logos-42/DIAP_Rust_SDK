// DIAP Rust SDK - 身份管理快速入门
// 最简单的 DID/IPNS 注册和验证示例

use diap_rs_sdk::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 DIAP 身份管理快速入门\n");
    
    // 1. 初始化
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None, None, 30,
    );
    
    let ipns_publisher = IpnsPublisher::new(
        true, true,
        Some("http://localhost:5001".to_string()),
        365,
    );
    
    let identity_manager = IdentityManager::new(ipfs_client, ipns_publisher);
    
    // 2. 生成密钥
    let keypair = KeyPair::generate()?;
    println!("🔑 DID: {}\n", keypair.did);
    
    // 3. 准备智能体信息
    let agent_info = AgentInfo {
        name: "快速入门示例智能体".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "API".to_string(),
                endpoint: "https://api.example.com".to_string(),
            },
        ],
        description: Some("这是一个快速入门示例".to_string()),
        tags: Some(vec!["demo".to_string()]),
    };
    
    // 4. 一键注册身份
    println!("📝 注册身份...");
    let registration = identity_manager
        .register_identity(&agent_info, &keypair)
        .await?;
    
    println!("✅ 注册成功！");
    println!("  IPNS: {}\n", registration.ipns_name);
    
    // 5. 一键验证身份
    println!("🔍 验证身份...");
    let verification = identity_manager
        .verify_identity(&registration.ipns_name)
        .await?;
    
    if verification.is_valid {
        println!("✅ 验证成功！");
        println!("  智能体: {}", verification.agent_info.name);
    } else {
        println!("❌ 验证失败");
    }
    
    Ok(())
}

