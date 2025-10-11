// DIAP Rust SDK - 统一身份管理演示
// 展示如何使用 IdentityManager 简化 DID/IPNS 注册和验证流程

use diap_rs_sdk::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("╔════════════════════════════════════════════════════╗");
    println!("║   DIAP Rust SDK - 统一身份管理演示                ║");
    println!("║   一键完成 DID/IPNS 注册和验证                     ║");
    println!("╚════════════════════════════════════════════════════╝\n");
    
    // ═══════════════════════════════════════════════════════════
    // 第一步：初始化身份管理器
    // ═══════════════════════════════════════════════════════════
    println!("【第一步】初始化身份管理器");
    println!("─────────────────────────────────────────────────");
    
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None,  // Pinata API key (可选)
        None,  // Pinata API secret (可选)
        30,    // 超时时间（秒）
    );
    
    let ipns_publisher = IpnsPublisher::new(
        true,  // 使用w3name
        true,  // 使用IPFS节点
        Some("http://localhost:5001".to_string()),
        365,   // IPNS记录有效期（天）
    );
    
    let identity_manager = IdentityManager::new(ipfs_client, ipns_publisher);
    println!("✓ 身份管理器初始化完成\n");
    
    // ═══════════════════════════════════════════════════════════
    // 第二步：生成密钥对
    // ═══════════════════════════════════════════════════════════
    println!("【第二步】生成密钥对");
    println!("─────────────────────────────────────────────────");
    
    let keypair = KeyPair::generate()?;
    println!("✓ 密钥对生成成功");
    println!("  DID: {}", keypair.did);
    println!("  IPNS Name: {}\n", keypair.ipns_name);
    
    // ═══════════════════════════════════════════════════════════
    // 第三步：准备智能体信息
    // ═══════════════════════════════════════════════════════════
    println!("【第三步】准备智能体信息");
    println!("─────────────────────────────────────────────────");
    
    let agent_info = AgentInfo {
        name: "我的去中心化智能体".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "API".to_string(),
                endpoint: "https://api.myagent.com".to_string(),
            },
            ServiceInfo {
                service_type: "WebSocket".to_string(),
                endpoint: "wss://ws.myagent.com".to_string(),
            },
            ServiceInfo {
                service_type: "DIAP".to_string(),
                endpoint: "https://diap.myagent.com/v1".to_string(),
            },
        ],
        description: Some("一个强大的去中心化智能体，支持多种协议和服务".to_string()),
        tags: Some(vec![
            "AI".to_string(), 
            "DeFi".to_string(), 
            "Web3".to_string()
        ]),
    };
    
    println!("✓ 智能体信息准备完成");
    println!("  名称: {}", agent_info.name);
    println!("  服务数量: {}", agent_info.services.len());
    println!("  标签: {:?}\n", agent_info.tags);
    
    // ═══════════════════════════════════════════════════════════
    // 第四步：一键注册身份（DID + IPFS + IPNS）
    // ═══════════════════════════════════════════════════════════
    println!("【第四步】一键注册身份");
    println!("─────────────────────────────────────────────────");
    println!("开始注册流程...");
    println!("  ▸ 构建 DID 文档");
    println!("  ▸ 上传到 IPFS");
    println!("  ▸ 注册 IPNS name");
    println!("  ▸ 添加 IPNS 引用");
    println!("  ▸ 更新并重新发布");
    
    let registration = match identity_manager
        .register_identity(&agent_info, &keypair)
        .await 
    {
        Ok(reg) => {
            println!("\n✅ 身份注册成功！");
            println!("╔════════════════════════════════════════════════════╗");
            println!("║  DID: {:<45}║", reg.did);
            println!("║  IPNS: {:<44}║", reg.ipns_name);
            println!("║  CID: {:<45}║", reg.cid);
            println!("╚════════════════════════════════════════════════════╝");
            reg
        }
        Err(e) => {
            println!("\n❌ 注册失败: {}", e);
            println!("\n提示：请确保 IPFS 节点正在运行：");
            println!("  $ ipfs daemon");
            return Err(e);
        }
    };
    
    println!("\n💾 保存密钥到本地文件...");
    let key_path = std::path::PathBuf::from("./demo_identity.key");
    keypair.save_to_file(&key_path)?;
    println!("✓ 密钥已保存到: {:?}\n", key_path);
    
    // ═══════════════════════════════════════════════════════════
    // 第五步：一键验证身份（通过 IPNS name）
    // ═══════════════════════════════════════════════════════════
    println!("【第五步】一键验证身份（通过 IPNS）");
    println!("─────────────────────────────────────────────────");
    println!("开始验证流程...");
    println!("  ▸ IPNS 解析");
    println!("  ▸ 下载 DID 文档");
    println!("  ▸ 验证签名和完整性");
    
    let _verification = match identity_manager
        .verify_identity(&registration.ipns_name)
        .await 
    {
        Ok(ver) => {
            println!("\n验证结果: {}", if ver.is_valid { "✅ 有效" } else { "❌ 无效" });
            println!("\n验证详情:");
            for detail in &ver.verification_details {
                println!("  {}", detail);
            }
            ver
        }
        Err(e) => {
            println!("\n❌ 验证失败: {}", e);
            return Err(e);
        }
    };
    
    // ═══════════════════════════════════════════════════════════
    // 第六步：通过 DID 直接验证
    // ═══════════════════════════════════════════════════════════
    println!("\n【第六步】通过 DID 直接验证");
    println!("─────────────────────────────────────────────────");
    
    let verification2 = identity_manager
        .resolve_by_did(&registration.did)
        .await?;
    
    println!("✓ DID 解析成功");
    println!("\n智能体详情:");
    println!("  名称: {}", verification2.agent_info.name);
    println!("  服务数量: {}", verification2.agent_info.services.len());
    
    if !verification2.agent_info.services.is_empty() {
        println!("\n  服务列表:");
        for (i, service) in verification2.agent_info.services.iter().enumerate() {
            println!("    {}. {} → {}", i + 1, service.service_type, service.endpoint);
        }
    }
    
    // ═══════════════════════════════════════════════════════════
    // 第七步：演示身份更新（可选）
    // ═══════════════════════════════════════════════════════════
    println!("\n【第七步】演示身份更新");
    println!("─────────────────────────────────────────────────");
    println!("添加新的服务端点...");
    
    let mut updated_agent_info = agent_info.clone();
    updated_agent_info.services.push(ServiceInfo {
        service_type: "GraphQL".to_string(),
        endpoint: "https://graphql.myagent.com".to_string(),
    });
    
    // 从原始注册结果获取序列号
    let current_sequence = if let Some(metadata) = &registration.did_document.ipfs_metadata {
        metadata.sequence
    } else {
        1
    };
    
    match identity_manager
        .update_identity(&updated_agent_info, &keypair, current_sequence)
        .await 
    {
        Ok(updated_reg) => {
            println!("✅ 身份更新成功");
            println!("  新 CID: {}", updated_reg.cid);
            println!("  服务数量: {}", updated_agent_info.services.len());
        }
        Err(e) => {
            println!("⚠️  身份更新失败: {}", e);
            println!("（这是可选功能，失败不影响演示）");
        }
    }
    
    // ═══════════════════════════════════════════════════════════
    // 总结
    // ═══════════════════════════════════════════════════════════
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║                   演示完成！                        ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!("\n📊 流程总结：");
    println!("  1. ✓ 初始化身份管理器");
    println!("  2. ✓ 生成密钥对");
    println!("  3. ✓ 准备智能体信息");
    println!("  4. ✓ 一键注册身份 (DID + IPFS + IPNS)");
    println!("  5. ✓ 一键验证身份 (通过 IPNS)");
    println!("  6. ✓ 通过 DID 直接验证");
    println!("  7. ✓ 演示身份更新");
    
    println!("\n🎯 核心优势：");
    println!("  • 只需调用 register_identity() 完成所有注册步骤");
    println!("  • 只需调用 verify_identity() 完成所有验证步骤");
    println!("  • DID ↔ IPNS ↔ CID 自动绑定");
    println!("  • 双层验证自动完成");
    
    println!("\n💡 下一步：");
    println!("  • 密钥文件: {:?}", key_path);
    println!("  • DID: {}", registration.did);
    println!("  • IPNS: {}", registration.ipns_name);
    println!("  • 可以在其他程序中使用这些信息进行身份验证");
    
    Ok(())
}

