// DIAP Rust SDK - 两个节点PubSub通信演示
// 展示两个节点如何通过PubSub进行认证消息通信

use anyhow::Result;
use diap_rs_sdk::*;
use libp2p::PeerId;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    log::info!("🚀 启动两个节点PubSub通信演示");
    
    // 创建两个节点的身份
    let (node1_keypair, node1_identity, node1_peer_id) = create_node_identity("节点1")?;
    let (node2_keypair, node2_identity, node2_peer_id) = create_node_identity("节点2")?;
    
    println!("节点1 - DID: {}, PeerID: {}", node1_keypair.did, node1_peer_id);
    println!("节点2 - DID: {}, PeerID: {}", node2_keypair.did, node2_peer_id);
    
    // 初始化IPFS客户端
    let (ipfs_client, _ipfs_manager) = IpfsClient::new_builtin_only(None, 30).await?;
    
    // 创建两个节点的网络管理器
    let mut node1_manager = create_network_manager(
        node1_identity,
        node1_keypair.clone(),
        node1_peer_id,
        "/ip4/0.0.0.0/tcp/4001".to_string(),
        ipfs_client.clone(),
    ).await?;
    
    let mut node2_manager = create_network_manager(
        node2_identity,
        node2_keypair.clone(),
        node2_peer_id,
        "/ip4/0.0.0.0/tcp/4002".to_string(),
        ipfs_client.clone(),
    ).await?;
    
    // 启动两个节点
    node1_manager.start().await?;
    node2_manager.start().await?;
    
    // 订阅主题
    let topic = "diap-test-channel";
    node1_manager.subscribe_topic(topic)?;
    node2_manager.subscribe_topic(topic)?;
    
    println!("✅ 两个节点已启动并订阅主题: {}", topic);
    
    // 发布包含PubSub信息的DID
    let node1_cid = publish_did_with_pubsub(
        &node1_keypair,
        &node1_peer_id,
        vec![topic.to_string()],
        node1_manager.listeners(),
        &ipfs_client,
    ).await?;
    
    let node2_cid = publish_did_with_pubsub(
        &node2_keypair,
        &node2_peer_id,
        vec![topic.to_string()],
        node2_manager.listeners(),
        &ipfs_client,
    ).await?;
    
    println!("✅ 两个节点的DID已发布到IPFS");
    println!("  节点1 CID: {}", node1_cid);
    println!("  节点2 CID: {}", node2_cid);
    
    // 等待网络稳定
    sleep(Duration::from_secs(2)).await;
    
    // 创建消息通道
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // 启动节点1的消息发送任务
    let node1_tx = tx.clone();
    let node1_peer_id_str = node1_peer_id.to_string();
    tokio::spawn(async move {
        for i in 1..=5 {
            let message = format!("Hello from {}! Message #{}", node1_peer_id_str, i);
            if let Ok(message_id) = node1_manager.publish_message(topic, message.as_bytes()).await {
                println!("📤 节点1发送消息: {:?}", message_id);
                node1_tx.send(format!("节点1发送: {}", message)).unwrap();
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
    
    // 启动节点2的消息发送任务
    let node2_tx = tx.clone();
    let node2_peer_id_str = node2_peer_id.to_string();
    tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await; // 稍微延迟
        for i in 1..=5 {
            let message = format!("Hello from {}! Message #{}", node2_peer_id_str, i);
            if let Ok(message_id) = node2_manager.publish_message(topic, message.as_bytes()).await {
                println!("📤 节点2发送消息: {:?}", message_id);
                node2_tx.send(format!("节点2发送: {}", message)).unwrap();
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
    
    // 启动事件处理任务
    let node1_handle = tokio::spawn(async move {
        if let Err(e) = node1_manager.handle_events().await {
            log::error!("节点1事件处理错误: {}", e);
        }
    });
    
    let node2_handle = tokio::spawn(async move {
        if let Err(e) = node2_manager.handle_events().await {
            log::error!("节点2事件处理错误: {}", e);
        }
    });
    
    // 接收并显示消息
    let mut message_count = 0;
    while let Some(message) = rx.recv().await {
        println!("📨 收到消息: {}", message);
        message_count += 1;
        
        if message_count >= 10 { // 总共10条消息
            break;
        }
    }
    
    // 显示网络统计信息
    println!("\n📊 网络统计信息:");
    println!("节点1统计:");
    let node1_stats = node1_manager.get_network_stats();
    println!("  PeerID: {}", node1_stats.peer_id);
    println!("  监听地址: {:?}", node1_stats.listeners);
    println!("  连接节点数: {}", node1_stats.connected_peers);
    
    println!("节点2统计:");
    let node2_stats = node2_manager.get_network_stats();
    println!("  PeerID: {}", node2_stats.peer_id);
    println!("  监听地址: {:?}", node2_stats.listeners);
    println!("  连接节点数: {}", node2_stats.connected_peers);
    
    // 等待一段时间让消息处理完成
    sleep(Duration::from_secs(2)).await;
    
    // 取消任务
    node1_handle.abort();
    node2_handle.abort();
    
    println!("✅ 两个节点PubSub通信演示完成");
    Ok(())
}

/// 创建节点身份
fn create_node_identity(name: &str) -> Result<(KeyPair, LibP2PIdentity, PeerId)> {
    let keypair = KeyPair::generate()?;
    let libp2p_identity = LibP2PIdentity::generate()?;
    let peer_id = *libp2p_identity.peer_id();
    
    log::info!("创建{}身份: DID={}, PeerID={}", name, keypair.did, peer_id);
    Ok((keypair, libp2p_identity, peer_id))
}

/// 创建网络管理器
async fn create_network_manager(
    identity: LibP2PIdentity,
    keypair: KeyPair,
    peer_id: PeerId,
    listen_addr: String,
    ipfs_client: IpfsClient,
) -> Result<DIAPNetworkManager> {
    // 创建身份管理器
    let identity_manager = IdentityManager::new(ipfs_client)?;
    
    // 创建PubSub认证器
    let pubsub_authenticator = PubsubAuthenticator::new(
        identity_manager,
        None,
        None,
    );
    
    // 设置本地身份
    pubsub_authenticator.set_local_identity(
        keypair,
        peer_id,
        "temp_cid".to_string(),
    ).await?;
    
    // 创建网络配置
    let network_config = DIAPNetworkConfig {
        listen_addrs: vec![listen_addr],
        bootstrap_peers: vec![],
        gossipsub_config: GossipsubConfig::default(),
        enable_mdns: true,
        enable_kad: true,
        protocol_version: "/diap/1.0.0".to_string(),
    };
    
    // 创建网络管理器
    DIAPNetworkManager::new(
        identity,
        network_config,
        Some(pubsub_authenticator),
    ).await
}

/// 发布包含PubSub信息的DID
async fn publish_did_with_pubsub(
    keypair: &KeyPair,
    peer_id: &PeerId,
    topics: Vec<String>,
    listeners: Vec<libp2p::Multiaddr>,
    ipfs_client: &IpfsClient,
) -> Result<String> {
    let mut did_builder = DIDBuilder::new(ipfs_client.clone());
    
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
        keypair,
        peer_id,
        topics,
        listeners.iter().map(|addr| addr.to_string()).collect(),
    ).await?;
    
    Ok(publish_result.cid)
}
