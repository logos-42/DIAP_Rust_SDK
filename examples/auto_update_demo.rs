// ANP Rust SDK - 自动更新示例
// 演示如何使用自动更新管理器定期刷新IPNS

use anp_rs_sdk::{
    ANPConfig, KeyManager, IpfsClient, IpnsPublisher, DIDBuilder, AutoUpdateManager,
};
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== ANP 自动更新示例 ===\n");
    
    // 加载配置
    let config = ANPConfig::load()?;
    
    // 初始化密钥
    let key_manager = KeyManager::new(
        config.agent.private_key_path.parent().unwrap().to_path_buf()
    );
    let keypair = key_manager.load_or_generate(&config.agent.private_key_path)?;
    
    println!("DID: {}", keypair.did);
    
    // 初始化IPFS和IPNS
    let ipfs_client = IpfsClient::new(
        config.ipfs.aws_api_url.clone(),
        config.ipfs.aws_gateway_url.clone(),
        config.ipfs.pinata_api_key.clone(),
        config.ipfs.pinata_api_secret.clone(),
        config.ipfs.timeout_seconds,
    );
    
    let ipns_publisher = IpnsPublisher::new(
        config.ipns.use_w3name,
        config.ipns.use_ipfs_node,
        config.ipfs.aws_api_url.clone(),
        config.ipns.validity_days,
    );
    
    // 创建DID构建器
    let mut did_builder = DIDBuilder::new(
        config.agent.name.clone(),
        ipfs_client,
        ipns_publisher,
    );
    
    did_builder.add_service("AgentAPI", "https://agent.example.com/api");
    
    // 首次创建和发布DID
    println!("\n🚀 创建和发布DID...");
    let result = did_builder.create_and_publish(&keypair).await?;
    
    println!("✓ DID发布成功");
    println!("  ├─ DID: {}", result.did);
    println!("  ├─ CID: {}", result.current_cid);
    println!("  └─ 序列号: {}", result.sequence);
    
    // 创建自动更新管理器
    // 注意：为了演示，这里设置为每10秒更新一次（实际应该是24小时）
    println!("\n⏰ 创建自动更新管理器");
    println!("  更新间隔: 10秒（演示用，实际应该是24小时）");
    
    let update_manager = AutoUpdateManager::new(
        did_builder,
        keypair,
        result.sequence,
        result.current_cid,
        10 / 3600,  // 10秒转换为小时（演示用）
    );
    
    // 启动自动更新
    println!("\n▶️  启动自动更新...");
    update_manager.start().await;
    
    println!("✓ 自动更新已启动");
    println!("\n等待更新...");
    println!("（按Ctrl+C停止）\n");
    
    // 运行30秒，观察自动更新
    for i in 1..=30 {
        sleep(Duration::from_secs(1)).await;
        
        if i % 10 == 0 {
            // 每10秒显示一次状态
            let state = update_manager.get_state().await;
            println!("📊 当前状态:");
            println!("  ├─ 序列号: {}", state.current_sequence);
            println!("  ├─ CID: {}", state.current_cid);
            println!("  ├─ 更新次数: {}", state.update_count);
            println!("  └─ 上次更新: {}\n", state.last_update);
        }
    }
    
    // 手动触发一次更新
    println!("🔄 手动触发更新...");
    match update_manager.trigger_update().await {
        Ok(result) => {
            println!("✓ 手动更新成功");
            println!("  └─ 新序列号: {}", result.sequence);
        }
        Err(e) => {
            println!("✗ 手动更新失败: {}", e);
        }
    }
    
    // 停止自动更新
    println!("\n⏹️  停止自动更新...");
    update_manager.stop().await;
    
    println!("\n✨ 示例完成！");
    println!("\n💡 实际使用建议:");
    println!("  - 设置更新间隔为24小时");
    println!("  - 在后台运行，无需用户干预");
    println!("  - 自动延长IPNS记录有效期");
    println!("  - 确保DID始终可解析");
    
    Ok(())
}

