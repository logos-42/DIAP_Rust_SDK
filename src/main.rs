/**
 * DIAP Rust SDK 主启动文件
 * Decentralized Intelligent Agent Protocol
 * 演示如何使用DIAP Rust SDK
 */

use diap_rs_sdk::{
    DIAPSDK, AutoConfigOptions, AgentInterface,
    diap_key_generator::{DIAPKeyGenerator, KeyType}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    println!("🚀 DIAP Rust SDK 启动演示");
    println!("================================");
    
    // 示例1: 基础密钥生成
    println!("\n📋 示例1: 基础密钥生成");
    basic_key_generation_example().await?;
    
    // 示例2: 完整DIAP智能体配置
    println!("\n📋 示例2: 完整DIAP智能体配置");
    full_diap_agent_example().await?;
    
    // 示例3: 自定义配置
    println!("\n📋 示例3: 自定义配置");
    custom_config_example().await?;
    
    println!("\n✅ 所有示例运行完成！");
    Ok(())
}

/// 基础密钥生成示例
async fn basic_key_generation_example() -> Result<(), Box<dyn std::error::Error>> {
    let generator = DIAPKeyGenerator::new("example.com".to_string(), Some("user:alice".to_string()));
    
    // 生成Ed25519密钥对
    let ed25519_result = generator.generate_keypair(KeyType::Ed25519)?;
    println!("✅ Ed25519 DID: {}", ed25519_result.did);
    println!("✅ Ed25519 私钥长度: {} 字符", ed25519_result.private_key.len());
    
    // 生成secp256k1密钥对
    let secp256k1_result = generator.generate_keypair(KeyType::Secp256k1)?;
    println!("✅ secp256k1 DID: {}", secp256k1_result.did);
    println!("✅ secp256k1 私钥长度: {} 字符", secp256k1_result.private_key.len());
    
    // 生成签名数据
    let signature_data = generator.generate_signature_data("example.com", &ed25519_result.did);
    println!("✅ 签名数据: nonce={}, timestamp={}", 
             signature_data.nonce, signature_data.timestamp);
    
    Ok(())
}

/// 完整DIAP智能体配置示例
async fn full_diap_agent_example() -> Result<(), Box<dyn std::error::Error>> {
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_port: Some(true),
        port_range: Some((3000, 3100)),
        agent_name: Some("Demo DIAP Agent".to_string()),
        interfaces: Some(vec![
            AgentInterface {
                interface_type: "NaturalLanguageInterface".to_string(),
                description: "Natural language processing interface".to_string(),
                url: None,
            },
            AgentInterface {
                interface_type: "StructuredInterface".to_string(),
                description: "Structured API interface".to_string(),
                url: None,
            }
        ]),
        log_level: Some("info".to_string()),
        ..Default::default()
    };

    let mut sdk = DIAPSDK::new(options);

    match sdk.start().await {
        Ok(config) => {
            println!("🎉 DIAP智能体启动成功！");
            println!("   - HTTP端点: {}", config.endpoint);
            println!("   - DID: {}", config.did);
            println!("   - 端口: {}", config.port);
            println!("   - 本地IP: {}", config.local_ip);
            
            // 测试健康检查
            let client = reqwest::Client::new();
            match client.get(&format!("{}/health", config.endpoint)).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let health: serde_json::Value = response.json().await?;
                        println!("✅ 健康检查通过: 状态={}", health["status"]);
                    }
                }
                Err(e) => {
                    println!("⚠️ 健康检查失败: {}", e);
                }
            }
            
            // 等待一段时间
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            
            sdk.stop().await?;
            println!("🛑 DIAP智能体已停止");
        }
        Err(e) => {
            eprintln!("❌ DIAP智能体启动失败: {}", e);
        }
    }

    Ok(())
}

/// 自定义配置示例
async fn custom_config_example() -> Result<(), Box<dyn std::error::Error>> {
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_port: Some(false), // 使用指定端口
        port_range: Some((8080, 8080)), // 指定端口8080
        agent_name: Some("Custom DIAP Agent".to_string()),
        interfaces: Some(vec![
            AgentInterface {
                interface_type: "StructuredInterface".to_string(),
                description: "Custom structured interface".to_string(),
                url: None,
            }
        ]),
        log_level: Some("debug".to_string()),
        ..Default::default()
    };

    let mut sdk = DIAPSDK::new(options);

    match sdk.start().await {
        Ok(config) => {
            println!("🎉 自定义DIAP智能体启动成功！");
            println!("   - 自定义端点: {}", config.endpoint);
            println!("   - DID: {}", config.did);
            println!("   - 指定端口: {}", config.port);
            
            // 测试配置端点
            let client = reqwest::Client::new();
            match client.get(&format!("{}/config", config.endpoint)).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let config_info: serde_json::Value = response.json().await?;
                        println!("✅ 配置信息获取成功: 端口={}", config_info["port"]);
                    }
                }
                Err(e) => {
                    println!("⚠️ 配置信息获取失败: {}", e);
                }
            }
            
            // 等待一段时间
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            sdk.stop().await?;
            println!("🛑 自定义DIAP智能体已停止");
        }
        Err(e) => {
            eprintln!("❌ 自定义DIAP智能体启动失败: {}", e);
        }
    }

    Ok(())
}
