/**
 * 完整 DIAP 智能体示例（包含 IPFS 注册）
 * 展示：DID 生成、did:web 支持、HTTP 路由、IPFS 注册表
 */

use diap_rs_sdk::{
    DIAPSDK, AutoConfigOptions, AgentInterface,
    IpfsRegistryConfig, IpfsRegistry, AgentSearchFilter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("\n🚀 完整 DIAP 智能体示例（包含 IPFS 注册）");
    println!("==========================================\n");
    
    // 配置选项
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_port: Some(true),
        auto_did: Some(true),
        auto_ipfs_register: Some(true), // 启用 IPFS 注册
        ipfs_config: Some(IpfsRegistryConfig {
            api_url: "http://127.0.0.1:5001".to_string(),
            gateway_url: "https://ipfs.io".to_string(),
            pin: true,
        }),
        port_range: Some((3000, 3100)),
        agent_name: Some("IPFS Demo Agent".to_string()),
        interfaces: Some(vec![
            AgentInterface {
                interface_type: "NaturalLanguageInterface".to_string(),
                description: "Natural language processing".to_string(),
                url: None,
            },
        ]),
        ..Default::default()
    };

    // 启动 SDK
    let mut sdk = DIAPSDK::new(options);
    
    match sdk.start().await {
        Ok(config) => {
            println!("✅ ANP 智能体启动成功！\n");
            println!("📋 配置信息:");
            println!("  - DID (wba): {}", config.did);
            if let Some(ref did_web) = config.did_web {
                println!("  - DID (web): {}", did_web);
            }
            println!("  - 端点: {}", config.endpoint);
            println!("  - 端口: {}", config.port);
            
            if let Some(ref ipfs_cid) = config.ipfs_cid {
                println!("\n🌐 IPFS 注册信息:");
                println!("  - CID: {}", ipfs_cid);
                println!("  - IPFS 网关: https://ipfs.io/ipfs/{}", ipfs_cid);
            }
            
            println!("\n📡 可访问的端点:");
            println!("  - 健康检查: {}/health", config.endpoint);
            println!("  - DID 文档: {}/.well-known/did.json", config.endpoint);
            println!("  - AD 文档: {}/agents/auto-agent/ad.json", config.endpoint);
            println!("  - ANP API: {}/anp/api", config.endpoint);
            
            // 测试 HTTP 端点
            println!("\n🧪 测试 HTTP 端点...");
            test_endpoints(&config.endpoint).await?;
            
            // 如果启用了 IPFS，演示查询功能
            if let Some(ref ipfs_cid) = config.ipfs_cid {
                println!("\n🔍 演示 IPFS 查询功能...");
                demo_ipfs_query(ipfs_cid).await?;
            }
            
            println!("\n⏳ 保持运行 30 秒...");
            println!("   (可以使用浏览器或 curl 访问上述端点)");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            
            println!("\n🛑 停止智能体...");
            sdk.stop().await?;
            println!("✅ 智能体已停止");
        }
        Err(e) => {
            eprintln!("❌ 启动失败: {}", e);
            eprintln!("\n💡 提示:");
            eprintln!("   - 如果启用了 IPFS 注册，请确保本地运行了 IPFS 节点");
            eprintln!("   - 可以关闭 IPFS 注册: auto_ipfs_register = false");
        }
    }

    Ok(())
}

/// 测试 HTTP 端点
async fn test_endpoints(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // 测试健康检查
    print!("  - 测试 /health ... ");
    match client.get(&format!("{}/health", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("✅");
            let health: serde_json::Value = resp.json().await?;
            println!("    状态: {}", health["status"]);
        }
        Ok(resp) => println!("❌ HTTP {}", resp.status()),
        Err(e) => println!("❌ {}", e),
    }
    
    // 测试 DID 文档
    print!("  - 测试 /.well-known/did.json ... ");
    match client.get(&format!("{}/.well-known/did.json", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("✅");
            let did_doc: serde_json::Value = resp.json().await?;
            println!("    DID: {}", did_doc["id"]);
        }
        Ok(resp) => println!("❌ HTTP {}", resp.status()),
        Err(e) => println!("❌ {}", e),
    }
    
    // 测试 AD 文档
    print!("  - 测试 /agents/auto-agent/ad.json ... ");
    match client.get(&format!("{}/agents/auto-agent/ad.json", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("✅");
            let ad: serde_json::Value = resp.json().await?;
            println!("    名称: {}", ad["name"]);
        }
        Ok(resp) => println!("❌ HTTP {}", resp.status()),
        Err(e) => println!("❌ {}", e),
    }
    
    Ok(())
}

/// 演示 IPFS 查询功能
async fn demo_ipfs_query(cid: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ipfs_config = IpfsRegistryConfig::default();
    let registry = IpfsRegistry::new(ipfs_config);
    
    println!("  - 从 IPFS 查询智能体信息...");
    match registry.query_agent(cid).await {
        Ok(entry) => {
            println!("    ✅ 查询成功");
            println!("    名称: {}", entry.name);
            println!("    DID: {}", entry.did);
            if let Some(ref did_web) = entry.did_web {
                println!("    DID (web): {}", did_web);
            }
            println!("    端点: {}", entry.endpoint);
            println!("    能力: {:?}", entry.capabilities);
            println!("    接口: {:?}", entry.interfaces);
        }
        Err(e) => {
            println!("    ⚠️ 查询失败: {}", e);
            println!("    (这可能需要等待 IPFS 网络传播)");
        }
    }
    
    Ok(())
}

