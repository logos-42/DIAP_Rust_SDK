// ANP Rust SDK - 批量上传示例
// 演示如何批量创建和上传多个DID

use diap_rs_sdk::{
    DIAPConfig, KeyPair, IpfsClient, IpnsPublisher, DIDBuilder, BatchUploader,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== ANP 批量上传示例 ===\n");
    
    // 加载配置
    let config = DIAPConfig::load()?;
    
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
    let did_builder = DIDBuilder::new(
        "Batch Agent".to_string(),
        ipfs_client,
        ipns_publisher,
    );
    
    // 创建批量上传器（最多10个并发）
    let batch_uploader = BatchUploader::new(did_builder, 10);
    
    println!("✓ 批量上传器初始化完成\n");
    
    // 准备批量上传的智能体
    println!("📦 准备批量上传项...");
    
    let mut items = Vec::new();
    
    // 生成3个测试智能体
    for i in 1..=3 {
        let agent_name = format!("TestAgent{}", i);
        let keypair = KeyPair::generate()?;
        
        println!("  ├─ {}: {}", agent_name, keypair.did);
        items.push((agent_name, keypair));
    }
    
    println!("\n🚀 开始批量上传（这可能需要一些时间）...\n");
    
    // 执行批量上传
    let start = std::time::Instant::now();
    let result = batch_uploader.batch_upload(items).await?;
    let elapsed = start.elapsed();
    
    // 显示结果
    println!("\n✅ 批量上传完成！");
    println!("  ├─ 总数: {}", result.success_count + result.failure_count);
    println!("  ├─ 成功: {}", result.success_count);
    println!("  ├─ 失败: {}", result.failure_count);
    println!("  ├─ 总耗时: {:.2}秒", result.total_duration);
    println!("  └─ 平均耗时: {:.2}秒/个", 
             result.total_duration / (result.success_count + result.failure_count) as f64);
    
    println!("\n📋 详细结果:");
    for item in &result.results {
        if item.success {
            println!("  ✓ {}", item.agent_name);
            println!("    ├─ DID: {}", item.did.as_ref().unwrap());
            println!("    ├─ CID: {}", item.cid.as_ref().unwrap());
            println!("    └─ 耗时: {:.2}秒", item.duration);
        } else {
            println!("  ✗ {}", item.agent_name);
            println!("    ├─ 错误: {}", item.error.as_ref().unwrap());
            println!("    └─ 耗时: {:.2}秒", item.duration);
        }
    }
    
    println!("\n💡 提示:");
    println!("  - 并发上传可以显著提升速度");
    println!("  - 当前并发数: 10");
    println!("  - 实际耗时: {:.2}秒（串行需要 ~{:.0}秒）", 
             elapsed.as_secs_f64(),
             result.total_duration);
    
    println!("\n✨ 示例完成！");
    
    Ok(())
}

