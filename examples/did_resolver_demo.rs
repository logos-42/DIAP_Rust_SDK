// ANP Rust SDK - DID解析示例
// 演示如何解析不同格式的DID

use anp_rs_sdk::{
    ANPConfig, IpfsClient, IpnsPublisher, DIDResolver,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== ANP DID解析器示例 ===\n");
    
    // 加载配置
    let config = ANPConfig::load()?;
    
    // 初始化IPFS客户端
    let ipfs_client = IpfsClient::new(
        config.ipfs.aws_api_url.clone(),
        config.ipfs.aws_gateway_url.clone(),
        config.ipfs.pinata_api_key.clone(),
        config.ipfs.pinata_api_secret.clone(),
        config.ipfs.timeout_seconds,
    );
    
    // 初始化IPNS发布器
    let ipns_publisher = IpnsPublisher::new(
        config.ipns.use_w3name,
        config.ipns.use_ipfs_node,
        config.ipfs.aws_api_url.clone(),
        config.ipns.validity_days,
    );
    
    // 创建DID解析器
    let resolver = DIDResolver::new(
        ipfs_client,
        ipns_publisher,
        30,
    );
    
    println!("✓ DID解析器初始化完成\n");
    
    // 示例1: 解析 did:ipfs 格式
    println!("📍 示例1: 解析 did:ipfs 格式");
    println!("请输入一个did:ipfs DID（或按回车跳过）:");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let did_ipfs = input.trim();
    
    if !did_ipfs.is_empty() && did_ipfs.starts_with("did:ipfs:") {
        println!("正在解析: {}", did_ipfs);
        match resolver.resolve(did_ipfs).await {
            Ok(result) => {
                println!("✓ 解析成功！");
                println!("  来源: {}", result.source);
                println!("  解析时间: {}", result.resolved_at);
                println!("\nDID文档:");
                println!("{}", serde_json::to_string_pretty(&result.did_document)?);
            }
            Err(e) => {
                println!("✗ 解析失败: {}", e);
            }
        }
    } else {
        println!("跳过did:ipfs示例\n");
    }
    
    // 示例2: 解析 did:wba 格式
    println!("\n📍 示例2: 解析 did:wba 格式");
    println!("请输入一个did:wba DID（或按回车跳过）:");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let did_wba = input.trim();
    
    if !did_wba.is_empty() && did_wba.starts_with("did:wba:") {
        println!("正在解析: {}", did_wba);
        match resolver.resolve(did_wba).await {
            Ok(result) => {
                println!("✓ 解析成功！");
                println!("  来源: {}", result.source);
                println!("  解析时间: {}", result.resolved_at);
                println!("\nDID文档:");
                println!("{}", serde_json::to_string_pretty(&result.did_document)?);
            }
            Err(e) => {
                println!("✗ 解析失败: {}", e);
            }
        }
    } else {
        println!("跳过did:wba示例\n");
    }
    
    // 示例3: 批量解析
    println!("\n📍 示例3: 批量解析");
    let test_dids = vec![
        // 添加你的测试DID
    ];
    
    if !test_dids.is_empty() {
        println!("正在批量解析 {} 个DID...", test_dids.len());
        let results = resolver.resolve_batch(test_dids).await;
        
        let mut success = 0;
        let mut failed = 0;
        
        for result in results {
            match result {
                Ok(_) => success += 1,
                Err(_) => failed += 1,
            }
        }
        
        println!("批量解析完成: 成功 {}, 失败 {}", success, failed);
    } else {
        println!("没有测试DID，跳过批量解析\n");
    }
    
    println!("\n✨ 示例完成！");
    
    Ok(())
}

