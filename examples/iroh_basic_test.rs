/**
 * Iroh基础功能测试
 * 基于真实Iroh API的基础通信测试
 */

use diap_rs_sdk::{
    IrohCommunicator, IrohConfig,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("🧪 Iroh基础功能测试");
    println!("==================");

    // 1. 创建Iroh通信器
    println!("\n🚀 创建Iroh通信器");
    let config = IrohConfig::default();
    let communicator = IrohCommunicator::new(config).await?;
    
    // 2. 获取节点地址
    let node_addr = communicator.get_node_addr()?;
    println!("✅ 节点地址: {}", node_addr);

    // 3. 创建测试消息
    println!("\n📝 创建测试消息");
    let test_message = communicator.create_heartbeat("did:test:alice");
    println!("✅ 心跳消息创建成功: {}", test_message.message_id);

    let auth_message = communicator.create_auth_request("did:test:alice", "did:test:bob", "test_challenge");
    println!("✅ 认证请求消息创建成功: {}", auth_message.message_id);

    // 4. 获取连接统计
    println!("\n📊 连接统计");
    let stats = communicator.get_connection_stats();
    for (key, value) in stats {
        println!("   {}: {}", key, value);
    }

    println!("\n✅ Iroh基础功能测试完成");
    println!("==================");
    println!("🎯 测试结果:");
    println!("   - ✅ Iroh通信器创建成功");
    println!("   - ✅ 节点地址获取成功");
    println!("   - ✅ 消息创建功能正常");
    println!("   - ✅ 统计信息获取正常");
    
    println!("\n💡 下一步:");
    println!("   - 实现多节点连接测试");
    println!("   - 实现消息发送和接收");
    println!("   - 集成到PubSub验证闭环");

    Ok(())
}
