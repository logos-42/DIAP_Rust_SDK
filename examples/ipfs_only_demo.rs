use diap_rs_sdk::{
    IpfsClient, 
    IpfsNodeManager,
    IpfsNodeConfig
};
use std::time::Instant;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 IPFS节点演示");
    println!("================================");
    
    // 启动内置IPFS节点
    println!("\n🚀 启动内置IPFS节点...");
    let ipfs_config = IpfsNodeConfig {
        data_dir: std::env::temp_dir().join("diap_ipfs_demo"),
        api_port: 5001,
        gateway_port: 8080,
        auto_start: true,
        startup_timeout: 30,
        enable_bootstrap: true,
        enable_swarm: true,
        swarm_port: 4001,
        verbose_logging: false,
    };
    
    let ipfs_manager = IpfsNodeManager::new(ipfs_config.clone());
    
    let start_time = Instant::now();
    ipfs_manager.start().await?;
    let startup_time = start_time.elapsed();
    
    println!("✅ IPFS节点启动成功");
    println!("   启动时间: {:?}", startup_time);
    println!("   API地址: {}", ipfs_manager.api_url());
    println!("   网关地址: {}", ipfs_manager.gateway_url());
    
    // 创建IPFS客户端
    println!("\n📡 创建IPFS客户端...");
    let (ipfs_client, _ipfs_manager) = IpfsClient::new_with_builtin_node(
        Some(ipfs_config.clone()), 
        None, 
        None, 
        None, 
        None, 
        30
    ).await?;
    
    println!("✅ IPFS客户端创建成功");
    
    // 测试IPFS节点状态
    println!("\n🔍 检查节点状态...");
    let status = ipfs_manager.status().await;
    println!("   节点状态: {:?}", status);
    
    // 获取节点信息
    println!("\n📊 获取节点信息...");
    match ipfs_manager.get_node_info().await {
        Ok(info) => {
            println!("✅ 节点信息获取成功");
            println!("   节点ID: {}", info.id);
            println!("   版本: {}", info.agent_version);
            println!("   协议版本: {}", info.protocol_version);
            println!("   公钥: {}", info.public_key);
        }
        Err(e) => {
            println!("❌ 节点信息获取失败: {}", e);
        }
    }
    
    println!("\n🎉 IPFS节点演示完成！");
    println!("================================");
    println!("✅ 成功演示了以下功能：");
    println!("   1. 内置IPFS节点启动");
    println!("   2. IPFS客户端创建");
    println!("   3. 节点状态检查");
    println!("   4. 节点信息获取");
    
    println!("\n💡 下一步可以：");
    println!("   1. 上传内容到IPFS");
    println!("   2. 创建智能体身份");
    println!("   3. 实现ZKP身份验证");
    
    Ok(())
}
