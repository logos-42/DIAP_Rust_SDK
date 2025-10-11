// DIAP Rust SDK - libp2p + DID 完整示例
// 演示如何使用libp2p实现完全去中心化的DID

use diap_rs_sdk::{
    DIAPConfig, KeyManager, LibP2PIdentityManager, LibP2PNode,
    IpfsClient, IpnsPublisher, StartupManager, StartupConfig,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== DIAP libp2p + DID 完整示例 ===\n");
    
    // 步骤1: 加载配置
    println!("📋 步骤1: 加载配置");
    let config = DIAPConfig::load()?;
    println!("✓ 配置加载成功\n");
    
    // 步骤2: 初始化IPNS密钥（用于DID标识）
    println!("🔑 步骤2: 初始化IPNS密钥");
    let key_manager = KeyManager::new(
        config.agent.private_key_path.parent().unwrap().to_path_buf()
    );
    let ipns_keypair = key_manager.load_or_generate(&config.agent.private_key_path)?;
    
    println!("✓ IPNS密钥加载成功");
    println!("  DID: {}", ipns_keypair.did);
    println!("  IPNS名称: {}\n", ipns_keypair.ipns_name);
    
    // 步骤3: 初始化libp2p身份（用于P2P通信）
    println!("🌐 步骤3: 初始化libp2p身份");
    let libp2p_key_path = config.agent.private_key_path
        .parent()
        .unwrap()
        .join("libp2p.key");
    
    let libp2p_manager = LibP2PIdentityManager::new(
        libp2p_key_path.parent().unwrap().to_path_buf()
    );
    let libp2p_identity = libp2p_manager.load_or_generate(&libp2p_key_path)?;
    
    println!("✓ libp2p身份加载成功");
    println!("  PeerID: {}", libp2p_identity.peer_id_string());
    println!("  注意: PeerID ≠ IPNS名称（两个独立的密钥对）\n");
    
    // 步骤4: 创建libp2p节点
    println!("🏗️  步骤4: 创建libp2p节点");
    let mut node = LibP2PNode::new(&libp2p_identity)?;
    
    // 添加监听地址
    node.add_listen_addr("/ip4/0.0.0.0/tcp/4001")?;
    node.add_listen_addr("/ip6/::/tcp/4001")?;
    
    println!("✓ libp2p节点创建成功");
    
    // 获取节点信息
    let node_info = node.get_node_info();
    println!("  PeerID: {}", node_info.peer_id);
    println!("  多地址:");
    for addr in &node_info.multiaddrs {
        println!("    - {}", addr);
    }
    println!();
    
    // 步骤5: 初始化IPFS和IPNS客户端
    println!("📦 步骤5: 初始化IPFS和IPNS客户端");
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
    println!("✓ 客户端初始化完成\n");
    
    // 步骤6: 创建启动管理器
    println!("🚀 步骤6: 创建启动管理器");
    let startup_config = StartupConfig {
        always_update: true,
        address_freshness_threshold: 3600,
    };
    
    let startup_manager = StartupManager::new(
        ipns_keypair.clone(),
        libp2p_identity.clone(),
        ipfs_client,
        ipns_publisher,
        startup_config,
    );
    println!("✓ 启动管理器创建成功\n");
    
    // 步骤7: 启动时自动更新DID文档
    println!("⚡ 步骤7: 启动时自动更新DID文档");
    println!("   正在上传DID文档到IPFS并更新IPNS...");
    println!("   这可能需要几秒钟...\n");
    
    let result = startup_manager.update_on_startup(&node, None).await?;
    
    println!("\n✅ DID文档更新成功！\n");
    println!("📄 发布结果:");
    println!("  ├─ DID: {}", result.did);
    println!("  ├─ IPNS: /ipns/{}", result.ipns_name);
    println!("  ├─ CID: {}", result.current_cid);
    println!("  └─ 序列号: {}", result.sequence);
    
    println!("\n🔗 访问方式:");
    println!("  ├─ IPFS: https://ipfs.io/ipfs/{}", result.current_cid);
    println!("  └─ IPNS: https://ipfs.io/ipns/{}", result.ipns_name);
    
    println!("\n📋 DID文档内容:");
    let did_json = serde_json::to_string_pretty(&result.did_document)?;
    println!("{}", did_json);
    
    println!("\n🔍 关键信息说明:");
    println!("  1. DID标识符: {}", result.did);
    println!("     └─ 基于IPNS密钥派生（不是PeerID）");
    println!();
    println!("  2. IPNS名称: {}", result.ipns_name);
    println!("     └─ 用于内容寻址和DID解析");
    println!();
    println!("  3. PeerID: {}", node_info.peer_id);
    println!("     └─ 用于P2P通信（在DID文档的service字段中）");
    println!();
    println!("  4. 多地址: 在DID文档的LibP2PNode服务中");
    println!("     └─ 其他智能体可以通过这些地址直接连接");
    
    println!("\n💡 架构说明:");
    println!("  ┌─ IPNS Keypair ─→ DID标识符（did:ipfs:k51qzi5u...）");
    println!("  │                  └─ 用于身份认证和DID解析");
    println!("  │");
    println!("  └─ libp2p Keypair ─→ PeerID（12D3KooW...）");
    println!("                       └─ 用于P2P通信和连接");
    println!();
    println!("  两个密钥对独立管理，各司其职！");
    
    println!("\n✨ 示例完成！");
    println!("\n📚 下一步:");
    println!("  - 其他智能体可以通过DID解析获取你的PeerID和多地址");
    println!("  - 然后使用libp2p直接连接进行P2P通信");
    println!("  - 每次启动都会自动更新地址，确保可达性");
    
    Ok(())
}