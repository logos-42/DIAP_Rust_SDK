/**
 * 基础 DIAP 智能体示例（包含 did:web 支持）
 * 展示：DID 生成、did:web 格式、HTTP 路由、真实文档输出
 */

use diap_rs_sdk::{DIAPSDK, AutoConfigOptions, AgentInterface};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("\n🚀 基础 DIAP 智能体示例");
    println!("==========================\n");
    
    // 配置选项
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_port: Some(true),
        auto_did: Some(true),
        auto_ipfs_register: Some(false), // 不使用 IPFS
        port_range: Some((3000, 3100)),
        agent_name: Some("Demo Agent".to_string()),
        interfaces: Some(vec![
            AgentInterface {
                interface_type: "NaturalLanguageInterface".to_string(),
                description: "支持自然语言交互".to_string(),
                url: None,
            },
            AgentInterface {
                interface_type: "StructuredInterface".to_string(),
                description: "支持结构化 API 调用".to_string(),
                url: None,
            },
        ]),
        ..Default::default()
    };

    // 启动 SDK
    let mut sdk = DIAPSDK::new(options);
    
    match sdk.start().await {
        Ok(config) => {
            println!("✅ DIAP 智能体启动成功！\n");
            
            println!("📋 DID 信息:");
            println!("  - DID (wba 格式): {}", config.did);
            if let Some(ref did_web) = config.did_web {
                println!("  - DID (web 格式): {}", did_web);
            }
            
            println!("\n🌐 服务信息:");
            println!("  - 端点: {}", config.endpoint);
            println!("  - 端口: {}", config.port);
            println!("  - 本地 IP: {}", config.local_ip);
            
            println!("\n📡 可访问的 HTTP 端点:");
            println!("  ┌─ 健康检查");
            println!("  │  GET {}/health", config.endpoint);
            println!("  │");
            println!("  ┌─ DID 文档（符合 W3C DID 规范）");
            println!("  │  GET {}/.well-known/did.json", config.endpoint);
            println!("  │");
            println!("  ┌─ 智能体描述（符合 DIAP 规范）");
            println!("  │  GET {}/agents/auto-agent/ad.json", config.endpoint);
            println!("  │");
            println!("  └─ DIAP 通信端点");
            println!("     POST {}/anp/api", config.endpoint);
            
            println!("\n💡 使用方法:");
            println!("  # 使用 curl 测试");
            println!("  curl {}/health", config.endpoint);
            println!("  curl {}/.well-known/did.json", config.endpoint);
            println!("  curl {}/agents/auto-agent/ad.json", config.endpoint);
            println!();
            println!("  # 使用浏览器访问");
            println!("  浏览器打开: {}/health", config.endpoint);
            
            // 自动测试端点
            println!("\n🧪 自动测试端点...\n");
            test_all_endpoints(&config.endpoint).await?;
            
            println!("\n⏳ 保持运行 60 秒，您可以尝试访问上述端点...");
            println!("   按 Ctrl+C 可提前停止\n");
            
            // 保持运行
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            
            println!("\n🛑 停止智能体...");
            sdk.stop().await?;
            println!("✅ 智能体已停止");
        }
        Err(e) => {
            eprintln!("❌ 启动失败: {}", e);
        }
    }

    Ok(())
}

/// 测试所有端点
async fn test_all_endpoints(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // 1. 健康检查
    println!("1️⃣  测试健康检查端点");
    match client.get(&format!("{}/health", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let health: serde_json::Value = resp.json().await?;
            println!("   ✅ 状态: {}", health["status"]);
            println!("   📊 响应: {}\n", serde_json::to_string_pretty(&health)?);
        }
        Ok(resp) => println!("   ❌ HTTP {}\n", resp.status()),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }
    
    // 2. DID 文档
    println!("2️⃣  测试 DID 文档端点");
    match client.get(&format!("{}/.well-known/did.json", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let did_doc: serde_json::Value = resp.json().await?;
            println!("   ✅ DID: {}", did_doc["id"]);
            println!("   🔑 验证方法数量: {}", did_doc["verificationMethod"].as_array().map(|v| v.len()).unwrap_or(0));
            println!("   📄 完整文档:");
            println!("{}\n", serde_json::to_string_pretty(&did_doc)?);
        }
        Ok(resp) => println!("   ❌ HTTP {}\n", resp.status()),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }
    
    // 3. AD 文档
    println!("3️⃣  测试智能体描述端点");
    match client.get(&format!("{}/agents/auto-agent/ad.json", base_url)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let ad: serde_json::Value = resp.json().await?;
            println!("   ✅ 名称: {}", ad["name"]);
            println!("   📝 描述: {}", ad["description"]);
            println!("   🔧 接口数量: {}", ad["ad:interfaces"].as_array().map(|v| v.len()).unwrap_or(0));
            println!("   🎯 能力数量: {}", ad["ad:capabilities"].as_array().map(|v| v.len()).unwrap_or(0));
            println!("   📄 完整文档:");
            println!("{}\n", serde_json::to_string_pretty(&ad)?);
        }
        Ok(resp) => println!("   ❌ HTTP {}\n", resp.status()),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }
    
    // 4. DIAP API
    println!("4️⃣  测试 DIAP API 端点");
    let anp_request = serde_json::json!({
        "message": "Hello from DIAP SDK!",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    match client.post(&format!("{}/anp/api", base_url))
        .json(&anp_request)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let anp_response: serde_json::Value = resp.json().await?;
            println!("   ✅ 响应接收");
            println!("   📄 响应内容:");
            println!("{}\n", serde_json::to_string_pretty(&anp_response)?);
        }
        Ok(resp) => println!("   ❌ HTTP {}\n", resp.status()),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }
    
    Ok(())
}

