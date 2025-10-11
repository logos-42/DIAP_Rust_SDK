// DIAP Rust SDK - IPFS/IPNS基础示例
// 演示如何使用新的IPFS/IPNS功能创建和发布DID

use diap_rs_sdk::{
    DIAPConfig, KeyManager, IpfsClient, IpnsPublisher, DIDBuilder,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();
    
    println!("=== DIAP IPFS/IPNS 基础示例 ===\n");
    
    // 步骤1: 加载配置
    println!("📋 步骤1: 加载配置");
    let config = DIAPConfig::load()?;
    println!("✓ 配置加载成功\n");
    
    // 步骤2: 初始化密钥管理器
    println!("🔑 步骤2: 初始化密钥");
    let key_manager = KeyManager::new(
        config.agent.private_key_path.parent().unwrap().to_path_buf()
    );
    
    let keypair = key_manager.load_or_generate(&config.agent.private_key_path)?;
    println!("✓ 密钥加载成功");
    println!("  DID: {}", keypair.did);
    println!("  IPNS名称: {}\n", keypair.ipns_name);
    
    // 步骤3: 初始化IPFS客户端
    println!("📦 步骤3: 初始化IPFS客户端");
    let ipfs_client = IpfsClient::new(
        config.ipfs.aws_api_url.clone(),
        config.ipfs.aws_gateway_url.clone(),
        config.ipfs.pinata_api_key.clone(),
        config.ipfs.pinata_api_secret.clone(),
        config.ipfs.timeout_seconds,
    );
    println!("✓ IPFS客户端初始化完成\n");
    
    // 步骤4: 初始化IPNS发布器
    println!("🌐 步骤4: 初始化IPNS发布器");
    let ipns_publisher = IpnsPublisher::new(
        config.ipns.use_w3name,
        config.ipns.use_ipfs_node,
        config.ipfs.aws_api_url.clone(),
        config.ipns.validity_days,
    );
    println!("✓ IPNS发布器初始化完成\n");
    
    // 步骤5: 创建DID构建器并添加服务
    println!("🏗️  步骤5: 构建DID文档");
    let mut did_builder = DIDBuilder::new(
        config.agent.name.clone(),
        ipfs_client,
        ipns_publisher,
    );
    
    // 添加服务端点
    did_builder
        .add_service("AgentWebSocket", "wss://agent.example.com/ws")
        .add_service("AgentAPI", "https://agent.example.com/api");
    
    println!("✓ DID构建器配置完成\n");
    
    // 步骤6: 创建并发布DID（双层验证）
    println!("🚀 步骤6: 创建并发布DID（双层验证流程）");
    println!("   这可能需要几秒钟...\n");
    
    let result = did_builder.create_and_publish(&keypair).await?;
    
    println!("\n✅ DID发布成功！\n");
    println!("📄 发布结果:");
    println!("  ├─ DID: {}", result.did);
    println!("  ├─ IPNS: /ipns/{}", result.ipns_name);
    println!("  ├─ CID: {}", result.current_cid);
    println!("  └─ 序列号: {}", result.sequence);
    
    println!("\n🔗 访问方式:");
    println!("  ├─ IPFS: https://ipfs.io/ipfs/{}", result.current_cid);
    println!("  └─ IPNS: https://ipfs.io/ipns/{}", result.ipns_name);
    
    println!("\n📋 DID文档内容:");
    println!("{}", serde_json::to_string_pretty(&result.did_document)?);
    
    // 步骤7: 验证双层一致性
    println!("\n🔍 步骤7: 验证双层一致性");
    let verification = anp_rs_sdk::verify_double_layer(
        &result.did_document,
        &result.ipns_name,
    )?;
    println!("✓ 双层验证通过: {}", verification);
    
    println!("\n✨ 示例完成！");
    
    Ok(())
}

