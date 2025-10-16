// DIAP Rust SDK - PubSub通信演示
// 展示如何使用libp2p NetworkBehaviour和PubSub功能

use anyhow::Result;
use diap_rs_sdk::*;
use libp2p::PeerId;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    log::info!("🚀 启动DIAP PubSub通信演示");
    
    // 1. 创建身份和密钥
    log::info!("步骤1: 创建身份和密钥");
    let keypair = KeyPair::generate()?;
    let libp2p_identity = LibP2PIdentity::generate()?;
    let peer_id = *libp2p_identity.peer_id();
    
    println!("DID: {}", keypair.did);
    println!("PeerID: {}", peer_id);
    
    // 2. 初始化IPFS客户端
    log::info!("步骤2: 初始化IPFS客户端");
    let (ipfs_client, _ipfs_manager) = IpfsClient::new_builtin_only(None, 30).await?;
    
    // 3. 创建身份管理器
    log::info!("步骤3: 创建身份管理器");
    let identity_manager = IdentityManager::new(ipfs_client.clone())?;
    
    // 4. 创建PubSub认证器
    log::info!("步骤4: 创建PubSub认证器");
    let pubsub_authenticator = PubsubAuthenticator::new(
        identity_manager.clone(),
        None, // 使用默认nonce管理器
        None, // 使用默认DID缓存
    );
    
    // 设置本地身份
    pubsub_authenticator.set_local_identity(
        keypair.clone(),
        peer_id,
        "temp_cid".to_string(), // 临时CID，稍后会更新
    ).await?;
    
    // 5. 创建网络配置
    log::info!("步骤5: 创建网络配置");
    let network_config = DIAPNetworkConfig {
        listen_addrs: vec![
            "/ip4/0.0.0.0/tcp/4001".to_string(),
            "/ip6/::/tcp/4001".to_string(),
        ],
        bootstrap_peers: vec![],
        gossipsub_config: GossipsubConfig::default(),
        enable_mdns: true,
        enable_kad: true,
        protocol_version: "/diap/1.0.0".to_string(),
    };
    
    // 6. 创建网络管理器
    log::info!("步骤6: 创建网络管理器");
    let mut network_manager = DIAPNetworkManager::new(
        libp2p_identity,
        network_config,
        Some(pubsub_authenticator),
    ).await?;
    
    // 7. 启动网络管理器
    log::info!("步骤7: 启动网络管理器");
    network_manager.start().await?;
    
    // 8. 订阅主题
    log::info!("步骤8: 订阅主题");
    let topics = vec![
        "diap-agent-announcements".to_string(),
        "diap-verification-requests".to_string(),
        "diap-status-updates".to_string(),
    ];
    
    for topic in &topics {
        network_manager.subscribe_topic(topic)?;
    }
    
    // 9. 创建DID构建器并发布包含PubSub信息的DID
    log::info!("步骤9: 发布包含PubSub信息的DID");
    let mut did_builder = DIDBuilder::new(ipfs_client);
    
    // 添加API服务
    did_builder.add_service(
        "API",
        serde_json::json!({
            "endpoint": "https://api.example.com",
            "version": "1.0.0"
        })
    );
    
    // 发布包含PubSub信息的DID
    let publish_result = did_builder.create_and_publish_with_pubsub(
        &keypair,
        &peer_id,
        topics.clone(),
        network_manager.listeners().iter().map(|addr| addr.to_string()).collect(),
    ).await?;
    
    println!("✅ DID发布成功！");
    println!("  DID: {}", publish_result.did);
    println!("  CID: {}", publish_result.cid);
    println!("  PubSub主题: {:?}", topics);
    println!("  网络地址: {:?}", network_manager.listeners());
    
    // 10. 更新PubSub认证器的CID
    log::info!("步骤10: 更新PubSub认证器的CID");
    // 这里需要重新设置本地身份，包含正确的CID
    // 由于PubsubAuthenticator的set_local_identity方法需要重新创建，这里简化处理
    
    // 11. 发布一些测试消息
    log::info!("步骤11: 发布测试消息");
    for (i, topic) in topics.iter().enumerate() {
        let message = format!("Hello from DIAP agent! Message #{}", i + 1);
        let message_id = network_manager.publish_message(topic, message.as_bytes()).await?;
        println!("📤 发布消息到主题 '{}': {:?}", topic, message_id);
        sleep(Duration::from_millis(500)).await;
    }
    
    // 12. 显示网络统计信息
    log::info!("步骤12: 显示网络统计信息");
    let stats = network_manager.get_network_stats();
    println!("📊 网络统计信息:");
    println!("  PeerID: {}", stats.peer_id);
    println!("  监听地址: {:?}", stats.listeners);
    println!("  订阅主题: {:?}", stats.subscribed_topics);
    println!("  连接节点数: {}", stats.connected_peers);
    
    // 13. 运行事件循环（简化版）
    log::info!("步骤13: 运行事件循环");
    println!("🔄 网络管理器运行中，按Ctrl+C退出...");
    
    // 运行事件循环
    tokio::select! {
        _ = network_manager.handle_events() => {
            log::info!("事件循环结束");
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("收到退出信号");
        }
    }
    
    log::info!("✅ PubSub演示完成");
    Ok(())
}
