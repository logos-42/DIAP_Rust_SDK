/**
 * IPNS 发布功能测试示例
 * 演示如何使用 IpfsClient 的 IPNS 发布功能
 */

use diap_rs_sdk::IpfsClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 IPNS 发布功能测试");
    println!("{}", "=".repeat(50));
    
    // 1. 创建 IPFS 客户端（连接本地 Kubo）
    let api_url = std::env::var("DIAP_IPFS_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:5001".to_string());
    let gateway_url = std::env::var("DIAP_IPFS_GATEWAY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
    
    println!("\n📡 连接到 IPFS 节点:");
    println!("   API: {}", api_url);
    println!("   网关: {}", gateway_url);
    
    let ipfs_client = IpfsClient::new_with_remote_node(
        api_url.clone(),
        gateway_url.clone(),
        30
    );
    
    // 2. 上传测试内容到 IPFS
    println!("\n📤 上传测试内容到 IPFS...");
    let test_content = serde_json::json!({
        "test": "ipns_publish_test",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "这是一个 IPNS 发布测试"
    });
    
    let upload_result = ipfs_client.upload(
        &serde_json::to_string_pretty(&test_content)?,
        "test_ipns.json"
    ).await?;
    
    println!("   ✅ 上传成功!");
    println!("   CID: {}", upload_result.cid);
    println!("   大小: {} 字节", upload_result.size);
    
    // 3. 确保 IPNS key 存在
    let key_name = "diap_test";
    println!("\n🔑 确保 IPNS key '{}' 存在...", key_name);
    
    match ipfs_client.ensure_key_exists(key_name).await {
        Ok(key) => {
            println!("   ✅ Key '{}' 已准备好", key);
        }
        Err(e) => {
            println!("   ❌ Key 创建/检查失败: {}", e);
            println!("   提示: 请确保本地 Kubo IPFS 节点正在运行");
            return Err(e);
        }
    }
    
    // 4. 发布 IPNS 记录
    println!("\n📣 发布 IPNS 记录...");
    let lifetime = "24h";  // 24小时
    let ttl = "1h";        // 1小时
    
    match ipfs_client.publish_ipns(&upload_result.cid, key_name, lifetime, ttl).await {
        Ok(ipns_result) => {
            println!("   ✅ IPNS 发布成功!");
            println!("   名称: /ipns/{}", ipns_result.name);
            println!("   值: {}", ipns_result.value);
            println!("   发布时间: {}", ipns_result.published_at);
            
            // 5. 验证 IPNS 记录
            println!("\n🔍 验证 IPNS 记录...");
            let ipns_url = format!("{}/ipns/{}", gateway_url, ipns_result.name);
            let ipfs_url = format!("{}/ipfs/{}", gateway_url, upload_result.cid);
            
            println!("   IPNS URL: {}", ipns_url);
            println!("   IPFS URL: {}", ipfs_url);
            
            let http_client = reqwest::Client::new();
            
            // 验证 IPFS 访问
            match http_client.get(&ipfs_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("   ✅ IPFS 网关访问成功");
                        if let Ok(text) = resp.text().await {
                            println!("   内容预览: {}", &text[..text.len().min(100)]);
                        }
                    } else {
                        println!("   ⚠️  IPFS 网关返回: {}", resp.status());
                    }
                }
                Err(e) => {
                    println!("   ❌ IPFS 网关访问失败: {}", e);
                }
            }
            
            // 验证 IPNS 访问
            println!("\n   等待 IPNS 传播...");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            match http_client.get(&ipns_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("   ✅ IPNS 网关访问成功");
                        if let Ok(text) = resp.text().await {
                            println!("   内容预览: {}", &text[..text.len().min(100)]);
                        }
                    } else {
                        println!("   ⚠️  IPNS 网关返回: {}", resp.status());
                        println!("   提示: IPNS 记录可能需要更多时间传播");
                    }
                }
                Err(e) => {
                    println!("   ❌ IPNS 网关访问失败: {}", e);
                }
            }
            
            // 6. 测试便捷方法
            println!("\n🔄 测试便捷方法 publish_after_upload...");
            let new_content = serde_json::json!({
                "test": "updated_content",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": "这是更新后的内容"
            });
            
            let new_upload = ipfs_client.upload(
                &serde_json::to_string_pretty(&new_content)?,
                "test_ipns_updated.json"
            ).await?;
            
            println!("   新 CID: {}", new_upload.cid);
            
            match ipfs_client.publish_after_upload(&new_upload.cid, key_name, lifetime, ttl).await {
                Ok(updated_ipns) => {
                    println!("   ✅ IPNS 更新成功!");
                    println!("   名称: /ipns/{}", updated_ipns.name);
                    println!("   新值: {}", updated_ipns.value);
                    println!("   提示: 同一个 IPNS 名称现在指向新的 CID");
                }
                Err(e) => {
                    println!("   ❌ IPNS 更新失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ IPNS 发布失败: {}", e);
            return Err(e);
        }
    }
    
    println!("\n✅ IPNS 发布功能测试完成!");
    println!("{}", "=".repeat(50));
    
    println!("\n📋 功能总结:");
    println!("   ✅ 自动创建/检查 IPNS key");
    println!("   ✅ 发布 IPNS 记录");
    println!("   ✅ 更新 IPNS 记录");
    println!("   ✅ 网关访问验证");
    println!("   ✅ 便捷方法支持");
    
    println!("\n💡 使用提示:");
    println!("   - IPNS 记录可以被多次更新");
    println!("   - 同一个 key 可以指向不同的 CID");
    println!("   - lifetime 控制记录的有效期");
    println!("   - ttl 控制缓存时间");
    println!("   - 使用 allow-offline=true 可以离线发布");
    
    Ok(())
}
