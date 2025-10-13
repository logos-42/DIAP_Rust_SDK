// DIAP Rust SDK - IPFS Pubsub认证通讯演示
// 展示如何使用认证的发布/订阅进行安全的P2P通信

use diap_rs_sdk::*;
use anyhow::Result;
use libp2p::identity::Keypair as LibP2PKeypair;
use libp2p::PeerId;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("\n🌐 DIAP Pubsub认证通讯演示\n");
    println!("========================================");
    println!("功能特性：");
    println!("  ✓ 基于DID的身份认证");
    println!("  ✓ ZKP零知识证明验证");
    println!("  ✓ 防重放攻击（Nonce管理）");
    println!("  ✓ DID文档智能缓存");
    println!("  ✓ 消息内容签名验证");
    println!("========================================\n");
    
    // ==================== 第1步：初始化组件 ====================
    println!("📦 第1步：初始化系统组件");
    
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None, None, 30,
    );
    
    // 创建身份管理器
    let identity_manager = match IdentityManager::new_with_keys(
        ipfs_client.clone(),
        "zkp_proving.key",
        "zkp_verifying.key",
    ) {
        Ok(manager) => {
            println!("✓ 身份管理器已创建（ZKP已加载）");
            manager
        }
        Err(e) => {
            eprintln!("❌ 无法加载ZKP keys: {}", e);
            eprintln!("请先运行: cargo run --example zkp_setup_keys");
            return Err(e);
        }
    };
    
    // 创建Nonce管理器
    let nonce_manager = NonceManager::new(Some(300), Some(60));
    println!("✓ Nonce管理器已创建（防重放攻击）");
    
    // 创建DID文档缓存
    let did_cache = DIDCache::new(Some(3600), Some(1000));
    println!("✓ DID文档缓存已创建（TTL: 1小时）");
    
    // 创建Pubsub认证器
    let authenticator = PubsubAuthenticator::new(
        identity_manager.clone(),
        Some(nonce_manager),
        Some(did_cache),
    );
    println!("✓ Pubsub认证器已创建\n");
    
    // ==================== 第2步：创建身份 ====================
    println!("🔑 第2步：创建本地身份");
    
    let keypair = KeyPair::generate()?;
    let libp2p_keypair = LibP2PKeypair::generate_ed25519();
    let peer_id = PeerId::from(libp2p_keypair.public());
    
    println!("✓ DID: {}", keypair.did);
    println!("✓ PeerID: {}\n", peer_id);
    
    // ==================== 第3步：注册身份到IPFS ====================
    println!("📝 第3步：注册身份到IPFS");
    
    let agent_info = AgentInfo {
        name: "Pubsub演示节点".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "PubsubNode".to_string(),
                endpoint: serde_json::json!({
                    "protocol": "diap-pubsub/1.0.0",
                    "topics": ["demo", "announcements"]
                }),
            },
        ],
        description: Some("演示IPFS Pubsub认证通讯".to_string()),
        tags: Some(vec!["pubsub".to_string(), "demo".to_string()]),
    };
    
    let registration = identity_manager
        .register_identity(&agent_info, &keypair, &peer_id)
        .await?;
    
    println!("✅ 身份注册成功！");
    println!("  CID: {}\n", registration.cid);
    
    // 设置认证器的本地身份
    authenticator.set_local_identity(
        keypair.clone(),
        peer_id,
        registration.cid.clone(),
    ).await?;
    
    // ==================== 第4步：配置主题策略 ====================
    println!("⚙️  第4步：配置Pubsub主题策略");
    
    // 配置"demo"主题 - 允许所有认证用户
    authenticator.configure_topic(TopicConfig {
        name: "demo".to_string(),
        policy: TopicPolicy::AllowAuthenticated,
        require_zkp: true,
        require_signature: true,
    }).await?;
    
    println!("✓ 主题'demo'：允许所有认证用户");
    
    // 配置"announcements"主题 - 仅允许特定DID
    authenticator.configure_topic(TopicConfig {
        name: "announcements".to_string(),
        policy: TopicPolicy::AllowList(vec![keypair.did.clone()]),
        require_zkp: true,
        require_signature: true,
    }).await?;
    
    println!("✓ 主题'announcements'：仅允许白名单DID\n");
    
    // ==================== 第5步：创建认证消息 ====================
    println!("📨 第5步：创建认证消息");
    
    let message_content = b"Hello from DIAP Pubsub!";
    let auth_message = authenticator.create_authenticated_message(
        "demo",
        message_content,
    ).await?;
    
    println!("✅ 认证消息已创建");
    println!("  消息ID: {}", auth_message.message_id);
    println!("  主题: {}", auth_message.topic);
    println!("  发送者: {}", auth_message.from_did);
    println!("  Nonce: {}", auth_message.nonce);
    println!("  ZKP证明大小: {} 字节", auth_message.zkp_proof.len());
    println!("  签名大小: {} 字节\n", auth_message.signature.len());
    
    // ==================== 第6步：序列化消息 ====================
    println!("📦 第6步：序列化消息（准备传输）");
    
    let serialized = PubsubAuthenticator::serialize_message(&auth_message)?;
    println!("✓ 序列化后大小: {} 字节\n", serialized.len());
    
    // ==================== 第7步：反序列化并验证 ====================
    println!("🔍 第7步：接收并验证消息");
    
    let received_message = PubsubAuthenticator::deserialize_message(&serialized)?;
    println!("✓ 消息反序列化成功");
    
    let verification = authenticator.verify_message(&received_message).await?;
    
    println!("\n📊 验证结果：");
    println!("  状态: {}", if verification.verified { "✅ 通过" } else { "❌ 失败" });
    println!("  发送者: {}", verification.from_did);
    println!("\n验证步骤：");
    for detail in &verification.details {
        println!("  {}", detail);
    }
    println!();
    
    if verification.verified {
        let content_str = String::from_utf8_lossy(&received_message.content);
        println!("✅ 消息内容: \"{}\"", content_str);
    }
    
    // ==================== 第8步：测试重放攻击防护 ====================
    println!("\n🛡️  第8步：测试重放攻击防护");
    
    println!("尝试重复验证同一消息（模拟重放攻击）...");
    let replay_verification = authenticator.verify_message(&received_message).await?;
    
    if !replay_verification.verified {
        println!("✅ 重放攻击防护生效！");
        println!("详情:");
        for detail in &replay_verification.details {
            if detail.contains("重放") || detail.contains("已被使用") {
                println!("  {}", detail);
            }
        }
    } else {
        println!("⚠️  警告：重放攻击防护可能有问题");
    }
    
    // ==================== 第9步：显示统计信息 ====================
    println!("\n📊 第9步：系统统计信息");
    
    let cache_stats = authenticator.cache_stats();
    println!("DID文档缓存:");
    println!("  总条目: {}", cache_stats.total_entries);
    println!("  总命中次数: {}", cache_stats.total_hits);
    println!("  最大容量: {}", cache_stats.max_entries);
    
    let nonce_count = authenticator.nonce_count();
    println!("\nNonce管理器:");
    println!("  已记录nonce: {}", nonce_count);
    
    // ==================== 第10步：演示PeerID验证 ====================
    println!("\n🔐 第10步：PeerID签名验证");
    
    let encrypted_peer_id = identity_manager.extract_encrypted_peer_id(&registration.did_document)?;
    let is_valid = identity_manager.verify_peer_id(
        &registration.did_document,
        &encrypted_peer_id,
        &peer_id,
    )?;
    
    println!("PeerID归属验证: {}", if is_valid { "✅ 通过" } else { "❌ 失败" });
    println!("  验证方式: Ed25519签名");
    println!("  隐私保护: 只暴露PeerID哈希，不暴露明文");
    
    // ==================== 总结 ====================
    println!("\n========================================");
    println!("✅ 演示完成！\n");
    println!("核心功能展示：");
    println!("  ✓ DID身份注册与管理");
    println!("  ✓ ZKP零知识证明生成与验证");
    println!("  ✓ Pubsub消息认证（签名+ZKP）");
    println!("  ✓ 防重放攻击（Nonce管理）");
    println!("  ✓ DID文档智能缓存");
    println!("  ✓ PeerID签名验证（隐私保护）");
    println!("\n安全特性：");
    println!("  • 端到端身份验证");
    println!("  • 零知识证明保护隐私");
    println!("  • 消息内容签名防篡改");
    println!("  • Nonce防重放攻击");
    println!("  • 主题级别授权控制");
    println!("\n下一步：");
    println!("  - 集成libp2p gossipsub进行实际P2P通信");
    println!("  - 实现多节点网络拓扑");
    println!("  - 添加消息加密（端到端）");
    println!("  - 实现Iroh高效数据传输");
    println!("========================================\n");
    
    Ok(())
}

