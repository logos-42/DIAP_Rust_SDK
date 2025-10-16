/**
 * Iroh基础功能测试
 * 测试Iroh通信器的基本创建和配置功能
 */

use diap_rs_sdk::{
    IrohCommunicator, 
    iroh_communicator::IrohConfig,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    println!("🚀 开始Iroh基础功能测试");

    // 创建Iroh配置
    let config = IrohConfig {
        listen_addr: Some("0.0.0.0:0".parse().unwrap()),
        data_dir: None,
        max_connections: Some(100),
        connection_timeout: Some(30),
        enable_relay: Some(true),
        enable_nat_traversal: Some(true),
    };

    println!("📋 配置信息:");
    println!("   监听地址: {:?}", config.listen_addr);
    println!("   最大连接数: {:?}", config.max_connections);
    println!("   连接超时: {:?}秒", config.connection_timeout);

    // 创建Iroh通信器
    let communicator = IrohCommunicator::new(config).await?;
    
    // 获取节点地址
    let node_addr = communicator.get_node_addr()?;
    println!("✅ Iroh通信器创建成功!");
    println!("   节点地址: {}", node_addr);

    // 测试消息创建
    let auth_message = communicator.create_auth_request(
        "did:example:alice",
        "did:example:bob", 
        "challenge123"
    );
    
    println!("📝 测试消息创建:");
    println!("   消息ID: {}", auth_message.message_id);
    println!("   消息类型: {:?}", auth_message.message_type);
    println!("   发送者: {}", auth_message.from_did);
    println!("   接收者: {:?}", auth_message.to_did);

    let heartbeat_message = communicator.create_heartbeat("did:example:alice");
    println!("💓 心跳消息:");
    println!("   消息ID: {}", heartbeat_message.message_id);
    println!("   消息类型: {:?}", heartbeat_message.message_type);

    println!("🎯 Iroh基础功能测试完成!");
    println!("⚠️  注意: 当前实现是基础框架，完整P2P通信需要进一步研究NodeAddr构造");

    Ok(())
}