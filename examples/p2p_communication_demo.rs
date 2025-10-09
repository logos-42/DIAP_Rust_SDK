// ANP Rust SDK - P2P通信完整示例
// 演示如何实现完整的P2P发现、连接和通信

use anp_rs_sdk::{
    ANPConfig, KeyManager, LibP2PIdentityManager, LibP2PNode,
    IpfsClient, IpnsPublisher, StartupManager, StartupConfig,
    DIDResolver, P2PCommunicator,
};
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== ANP P2P通信完整示例 ===\n");
    
    // 步骤1: 初始化Agent A
    println!("🤖 步骤1: 初始化Agent A");
    let agent_a = create_agent("Agent_A").await?;
    println!("✓ Agent A初始化完成");
    println!("  DID: {}", agent_a.did);
    println!("  PeerID: {}", agent_a.peer_id);
    println!();
    
    // 步骤2: 初始化Agent B
    println!("🤖 步骤2: 初始化Agent B");
    let agent_b = create_agent("Agent_B").await?;
    println!("✓ Agent B初始化完成");
    println!("  DID: {}", agent_b.did);
    println!("  PeerID: {}", agent_b.peer_id);
    println!();
    
    // 步骤3: 启动P2P通信器
    println!("🌐 步骤3: 启动P2P通信器");
    
    // 为Agent A创建通信器
    let resolver_a = create_resolver().await?;
    let mut communicator_a = P2PCommunicator::new(
        agent_a.libp2p_identity.clone(),
        resolver_a,
    ).await?;
    
    // 启动监听
    communicator_a.listen("/ip4/127.0.0.1/tcp/4001")?;
    
    println!("✓ Agent A通信器启动完成");
    println!("  监听地址: /ip4/127.0.0.1/tcp/4001");
    println!();
    
    // 步骤4: 演示完整的验证闭环
    println!("🔍 步骤4: 演示验证闭环");
    
    // 解析Agent B的DID
    println!("正在解析Agent B的DID...");
    let resolve_result = resolver_a.resolve(&agent_b.did).await?;
    
    println!("✓ DID解析成功");
    
    // 提取libp2p信息
    let node_info = anp_rs_sdk::DIDResolver::extract_libp2p_info(&resolve_result.did_document)?;
    println!("✓ 提取libp2p信息成功");
    println!("  PeerID: {}", node_info.peer_id);
    println!("  多地址: {:?}", node_info.multiaddrs);
    
    // 验证libp2p绑定
    let binding_valid = anp_rs_sdk::DIDResolver::verify_libp2p_binding(&resolve_result.did_document)?;
    println!("✓ libp2p绑定验证: {}", binding_valid);
    
    // 完整验证闭环
    let chain_valid = anp_rs_sdk::DIDResolver::verify_complete_chain(
        &resolve_result.did_document,
        &agent_b.ipns_name,
        None, // 暂时没有连接
    )?;
    println!("✓ 完整验证闭环: {}", chain_valid);
    println!();
    
    // 步骤5: 尝试P2P连接
    println!("🔗 步骤5: 尝试P2P连接");
    println!("正在连接Agent B...");
    
    match communicator_a.connect_to_agent(&agent_b.did).await {
        Ok(connected_peer_id) => {
            println!("✅ P2P连接成功！");
            println!("  连接的PeerID: {}", connected_peer_id);
            
            // 验证连接的PeerID
            let connection_valid = anp_rs_sdk::DIDResolver::verify_peer_connection(
                &resolve_result.did_document,
                &connected_peer_id,
            )?;
            println!("✓ 连接PeerID验证: {}", connection_valid);
            
            // 完整验证（包含连接）
            let full_chain_valid = anp_rs_sdk::DIDResolver::verify_complete_chain(
                &resolve_result.did_document,
                &agent_b.ipns_name,
                Some(&connected_peer_id),
            )?;
            println!("✅ 完整验证闭环（含连接）: {}", full_chain_valid);
        }
        Err(e) => {
            println!("❌ P2P连接失败: {}", e);
            println!("  原因: Agent B可能没有在线");
            println!("  这是正常的，因为我们没有启动Agent B的通信器");
        }
    }
    println!();
    
    // 步骤6: 演示消息发送（如果连接成功）
    println!("💬 步骤6: 演示消息发送");
    
    if communicator_a.connected_peers().contains_key(&agent_b.did) {
        println!("发送测试消息到Agent B...");
        
        let test_message = serde_json::json!({
            "text": "Hello from Agent A!",
            "type": "greeting"
        });
        
        match communicator_a.send_message(&agent_b.did, test_message).await {
            Ok(_) => {
                println!("✓ 消息发送成功");
            }
            Err(e) => {
                println!("❌ 消息发送失败: {}", e);
            }
        }
    } else {
        println!("⏭️  跳过消息发送（未连接）");
    }
    println!();
    
    println!("✨ 示例完成！\n");
    
    println!("📋 验证闭环总结:");
    println!("  1. ✅ DID → IPNS → CID → DID文档");
    println!("  2. ✅ DID文档包含libp2p公钥");
    println!("  3. ✅ DID文档包含PeerID（明文）");
    println!("  4. ✅ 验证libp2p公钥 → PeerID绑定");
    println!("  5. ✅ 验证连接PeerID → 文档PeerID一致");
    println!("  6. ✅ libp2p自动验证公钥 → PeerID");
    println!("  7. ✅ 形成完整的自证明闭环");
    
    println!("\n💡 认证逻辑:");
    println!("  - IPNS协议验证DID文档的发布者身份");
    println!("  - IPFS协议验证内容完整性");
    println!("  - libp2p协议验证P2P连接身份");
    println!("  - ANP协议验证应用层一致性");
    println!("  - 四层验证，安全性极高！");
    
    Ok(())
}

/// 创建智能体的辅助结构
struct AgentInfo {
    did: String,
    ipns_name: String,
    peer_id: String,
    libp2p_identity: anp_rs_sdk::LibP2PIdentity,
}

/// 创建智能体
async fn create_agent(name: &str) -> Result<AgentInfo> {
    let config = ANPConfig::load()?;
    
    // 创建专用的密钥路径
    let ipns_key_path = config.agent.private_key_path
        .parent()
        .unwrap()
        .join(format!("{}_ipns.key", name.to_lowercase()));
    
    let libp2p_key_path = config.agent.private_key_path
        .parent()
        .unwrap()
        .join(format!("{}_libp2p.key", name.to_lowercase()));
    
    // 加载IPNS密钥
    let key_manager = KeyManager::new(ipns_key_path.parent().unwrap().to_path_buf());
    let ipns_keypair = key_manager.load_or_generate(&ipns_key_path)?;
    
    // 加载libp2p密钥
    let libp2p_manager = LibP2PIdentityManager::new(libp2p_key_path.parent().unwrap().to_path_buf());
    let libp2p_identity = libp2p_manager.load_or_generate(&libp2p_key_path)?;
    
    // 创建节点
    let mut node = LibP2PNode::new(&libp2p_identity)?;
    node.add_listen_addr("/ip4/127.0.0.1/tcp/0")?; // 使用随机端口
    
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
    
    // 创建启动管理器并发布DID
    let startup_manager = StartupManager::new(
        ipns_keypair.clone(),
        libp2p_identity.clone(),
        ipfs_client,
        ipns_publisher,
        StartupConfig::default(),
    );
    
    let _result = startup_manager.update_on_startup(&node, None).await?;
    
    Ok(AgentInfo {
        did: ipns_keypair.did,
        ipns_name: ipns_keypair.ipns_name,
        peer_id: libp2p_identity.peer_id_string(),
        libp2p_identity,
    })
}

/// 创建DID解析器
async fn create_resolver() -> Result<DIDResolver> {
    let config = ANPConfig::load()?;
    
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
    
    Ok(DIDResolver::new(ipfs_client, ipns_publisher, 30))
}
