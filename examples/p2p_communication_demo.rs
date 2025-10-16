// DIAP Rust SDK - P2P通信演示
// 展示两个节点之间的点对点请求-响应通信

use anyhow::Result;
use diap_rs_sdk::*;
use libp2p::PeerId;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    log::info!("🚀 启动P2P通信演示");
    
    // 创建两个节点的身份
    let (node1_keypair, node1_identity, node1_peer_id) = create_node_identity("节点1")?;
    let (node2_keypair, node2_identity, node2_peer_id) = create_node_identity("节点2")?;
    
    println!("节点1 - DID: {}, PeerID: {}", node1_keypair.did, node1_peer_id);
    println!("节点2 - DID: {}, PeerID: {}", node2_keypair.did, node2_peer_id);
    
    // 创建两个P2P通信器
    let mut node1_communicator = P2PCommunicator::new(node1_identity, node1_keypair).await?;
    let mut node2_communicator = P2PCommunicator::new(node2_identity, node2_keypair).await?;
    
    // 启动监听
    node1_communicator.listen("/ip4/0.0.0.0/tcp/5001")?;
    node2_communicator.listen("/ip4/0.0.0.0/tcp/5002")?;
    
    println!("✅ 两个P2P通信器已启动");
    
    // 等待监听地址分配
    sleep(Duration::from_secs(1)).await;
    
    // 获取监听地址
    let node1_listeners = node1_communicator.listeners();
    let node2_listeners = node2_communicator.listeners();
    
    println!("节点1监听地址: {:?}", node1_listeners);
    println!("节点2监听地址: {:?}", node2_listeners);
    
    // 连接两个节点
    if let Some(node1_addr) = node1_listeners.first() {
        node2_communicator.dial(node1_peer_id, node1_addr.clone())?;
        println!("📞 节点2连接到节点1");
    }
    
    // 等待连接建立
    sleep(Duration::from_secs(2)).await;
    
    // 启动事件处理任务
    let node1_handle = tokio::spawn(async move {
        if let Err(e) = node1_communicator.handle_events().await {
            log::error!("节点1事件处理错误: {}", e);
        }
    });
    
    let node2_handle = tokio::spawn(async move {
        if let Err(e) = node2_communicator.handle_events().await {
            log::error!("节点2事件处理错误: {}", e);
        }
    });
    
    // 等待连接稳定
    sleep(Duration::from_secs(1)).await;
    
    // 发送一些测试请求
    println!("\n📤 开始发送测试请求...");
    
    // 1. Ping请求
    println!("1. 发送Ping请求");
    let ping_request_id = node2_communicator.send_request(
        node1_peer_id,
        "ping",
        serde_json::json!({"message": "Hello from node2"}),
        &node1_communicator.local_did(),
    ).await?;
    println!("   Ping请求ID: {}", ping_request_id);
    
    sleep(Duration::from_secs(1)).await;
    
    // 2. 获取信息请求
    println!("2. 发送获取信息请求");
    let info_request_id = node2_communicator.send_request(
        node1_peer_id,
        "get_info",
        serde_json::json!({"request": "node_info"}),
        &node1_communicator.local_did(),
    ).await?;
    println!("   信息请求ID: {}", info_request_id);
    
    sleep(Duration::from_secs(1)).await;
    
    // 3. 未知请求类型（测试错误处理）
    println!("3. 发送未知请求类型");
    let unknown_request_id = node2_communicator.send_request(
        node1_peer_id,
        "unknown_type",
        serde_json::json!({"test": "unknown"}),
        &node1_communicator.local_did(),
    ).await?;
    println!("   未知请求ID: {}", unknown_request_id);
    
    // 等待响应处理
    sleep(Duration::from_secs(3)).await;
    
    // 显示统计信息
    println!("\n📊 通信统计信息:");
    println!("节点1:");
    println!("  DID: {}", node1_communicator.local_did());
    println!("  PeerID: {}", node1_communicator.local_peer_id());
    println!("  监听地址: {:?}", node1_communicator.listeners());
    
    println!("节点2:");
    println!("  DID: {}", node2_communicator.local_did());
    println!("  PeerID: {}", node2_communicator.local_peer_id());
    println!("  监听地址: {:?}", node2_communicator.listeners());
    
    // 等待一段时间让所有消息处理完成
    sleep(Duration::from_secs(2)).await;
    
    // 取消任务
    node1_handle.abort();
    node2_handle.abort();
    
    println!("✅ P2P通信演示完成");
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
