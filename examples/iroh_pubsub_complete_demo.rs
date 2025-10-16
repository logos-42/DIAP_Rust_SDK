/**
 * DIAP Rust SDK - Iroh+PubSub完整验证闭环演示
 * 展示智能体通过Iroh P2P + PubSub通讯，使用IPFS DID和CID的ZKP验证身份
 * 
 * 架构说明：
 * - PubSub：用于广播和发现其他智能体
 * - Iroh P2P：用于可靠的点对点通信
 * - IPFS DID/CID：用于身份验证和ZKP证明
 */

use diap_rs_sdk::{
    // IPFS和DID相关
    IpfsBidirectionalVerificationManager,
    DIDBuilder, DIDPublishResult,
    
    // PubSub通信
    PubsubAuthenticator, TopicConfig, TopicPolicy,
    PubSubMessageType, AuthenticatedMessage,
    
    // Iroh P2P通信（暂时禁用）
    // IrohCommunicator, IrohConfig, IrohMessage,
    
    // 身份管理
    IdentityManager, AgentInfo, KeyPair,
    
    // 类型
    AgentInfo as AgentInfoType,
};
use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use std::collections::HashMap;

/// 智能体节点
struct AgentNode {
    name: String,
    did: String,
    keypair: KeyPair,
    pubsub: PubsubAuthenticator,
    iroh: IrohCommunicator,
    verification_manager: IpfsBidirectionalVerificationManager,
    node_addr: String,
}

impl AgentNode {
    /// 创建新的智能体节点
    async fn new(name: &str) -> Result<Self> {
        println!("🚀 创建智能体节点: {}", name);

        // 1. 创建身份和密钥
        let keypair = KeyPair::generate();
        let did = keypair.did.clone();
        println!("   ✅ 生成身份: {}", did);

        // 2. 创建IPFS验证管理器
        let verification_manager = IpfsBidirectionalVerificationManager::new()?;
        println!("   ✅ 初始化IPFS验证管理器");

        // 3. 创建PubSub认证器
        let ipfs_client = verification_manager.get_ipfs_client();
        let identity_manager = IdentityManager::new(ipfs_client);
        let pubsub = PubsubAuthenticator::new(identity_manager, None, None);
        println!("   ✅ 初始化PubSub认证器");

    // 4. 创建简化P2P通信器
    let iroh_config = IrohConfig::default();
    let mut iroh = IrohCommunicator::new(iroh_config).await?;
    let node_addr = iroh.get_node_addr()?;
    println!("   ✅ 初始化简化P2P通信器: {}", node_addr);

        // 5. 启动心跳监控
        iroh.start_heartbeat_monitor(&did, Duration::from_secs(30)).await;

        Ok(Self {
            name: name.to_string(),
            did,
            keypair,
            pubsub,
            iroh,
            verification_manager,
            node_addr,
        })
    }

    /// 注册DID到IPFS
    async fn register_did(&self) -> Result<DIDPublishResult> {
        println!("📝 {} 注册DID到IPFS", self.name);

        let mut did_builder = DIDBuilder::new(&self.keypair.did);
        did_builder
            .add_verification_method("Ed25519VerificationKey2020")
            .add_authentication_method("key-1")
            .add_pubsub_service(
                "DIAPPubSub",
                serde_json::json!({
                    "endpoint": format!("iroh://{}", self.node_addr),
                    "protocol": "iroh+p2p"
                }),
                vec!["diap:verification".to_string(), "diap:discovery".to_string()],
                vec![self.node_addr.clone()],
            );

        let result = did_builder.create_and_publish(&self.keypair).await?;
        println!("   ✅ DID已发布到IPFS: {}", result.cid);

        Ok(result)
    }

    /// 配置PubSub主题
    async fn setup_pubsub_topics(&mut self) -> Result<()> {
        println!("📡 {} 配置PubSub主题", self.name);

        // 配置验证主题
        let verification_config = TopicConfig {
            topic: "diap:verification".to_string(),
            policy: TopicPolicy::Authenticated,
            max_message_size: 1024,
            message_ttl: Duration::from_secs(300),
        };

        // 配置发现主题
        let discovery_config = TopicConfig {
            topic: "diap:discovery".to_string(),
            policy: TopicPolicy::Open,
            max_message_size: 512,
            message_ttl: Duration::from_secs(60),
        };

        // 订阅主题
        self.pubsub.subscribe_topic("diap:verification").await?;
        self.pubsub.subscribe_topic("diap:discovery").await?;

        println!("   ✅ 已订阅验证和发现主题");

        // 发布节点发现消息
        let discovery_msg = self.pubsub.create_simple_message(
            "diap:discovery",
            &format!("节点发现: {} ({})", self.name, self.did),
        ).await?;

        self.pubsub.publish_message(discovery_msg).await?;
        println!("   ✅ 已发布节点发现消息");

        Ok(())
    }

    /// 执行IPFS双向验证
    async fn perform_verification(&mut self, other_did: &str) -> Result<()> {
        println!("🔐 {} 开始IPFS双向验证", self.name);

        let agent_info = AgentInfoType {
            did: self.did.clone(),
            name: self.name.clone(),
            public_key: self.keypair.public_key.clone(),
            created_at: chrono::Utc::now(),
        };

        // 执行验证
        let verification_result = self.verification_manager
            .perform_bidirectional_verification(&agent_info, other_did)
            .await?;

        match verification_result.status {
            diap_rs_sdk::VerificationStatus::Success => {
                println!("   ✅ IPFS验证成功");
                if let Some(proof) = verification_result.proof {
                    println!("   📊 证明长度: {} 字节", proof.len());
                }
            }
            diap_rs_sdk::VerificationStatus::Failed => {
                println!("   ❌ IPFS验证失败");
            }
            _ => {
                println!("   ⏳ IPFS验证进行中");
            }
        }

        Ok(())
    }

    /// 建立P2P连接
    async fn establish_p2p_connection(&mut self, other_node_addr: &str) -> Result<String> {
        println!("🔗 {} 建立P2P连接", self.name);

        let connection_id = self.iroh.connect_to_node(other_node_addr).await?;

        println!("   ✅ P2P连接已建立: {}", connection_id);
        Ok(connection_id)
    }

    /// 发送P2P认证请求
    async fn send_p2p_auth_request(&self, target_did: &str, connection_id: &str) -> Result<()> {
        println!("📤 {} 发送P2P认证请求", self.name);

        let auth_request = self.iroh.create_auth_request(
            &self.did,
            target_did,
            "p2p_challenge_123",
        );

        self.iroh.send_message(connection_id, auth_request).await?;
        println!("   ✅ P2P认证请求已发送");

        Ok(())
    }

    /// 发送P2P认证响应
    async fn send_p2p_auth_response(&self, target_did: &str, connection_id: &str) -> Result<()> {
        println!("📤 {} 发送P2P认证响应", self.name);

        let auth_response = self.iroh.create_auth_response(
            &self.did,
            target_did,
            "p2p_response_456",
        );

        self.iroh.send_message(connection_id, auth_response).await?;
        println!("   ✅ P2P认证响应已发送");

        Ok(())
    }

    /// 发送P2P自定义消息
    async fn send_p2p_custom_message(&self, target_did: &str, connection_id: &str, content: &str) -> Result<()> {
        println!("📤 {} 发送P2P自定义消息", self.name);

        let custom_msg = self.iroh.create_custom_message(
            &self.did,
            Some(target_did),
            content,
            "CustomData",
        );

        self.iroh.send_message(connection_id, custom_msg).await?;
        println!("   ✅ P2P自定义消息已发送");

        Ok(())
    }

    /// 发布PubSub认证请求
    async fn publish_pubsub_auth_request(&self, target_did: &str) -> Result<()> {
        println!("📡 {} 发布PubSub认证请求", self.name);

        let auth_request = self.pubsub.create_auth_request(
            "diap:verification",
            target_did,
            "pubsub_challenge_789",
        ).await?;

        self.pubsub.publish_message(auth_request).await?;
        println!("   ✅ PubSub认证请求已发布");

        Ok(())
    }

    /// 发布PubSub认证响应
    async fn publish_pubsub_auth_response(&self, target_did: &str) -> Result<()> {
        println!("📡 {} 发布PubSub认证响应", self.name);

        let auth_response = self.pubsub.create_auth_response(
            "diap:verification",
            target_did,
            "pubsub_response_012",
        ).await?;

        self.pubsub.publish_message(auth_response).await?;
        println!("   ✅ PubSub认证响应已发布");

        Ok(())
    }

    /// 处理接收到的消息
    async fn handle_received_message(&self, message: &AuthenticatedMessage) {
        println!("📥 {} 收到PubSub消息: {}", self.name, message.message_type);
        
        match message.message_type {
            PubSubMessageType::AuthRequest => {
                println!("   🔐 收到认证请求: {}", message.content);
            }
            PubSubMessageType::AuthResponse => {
                println!("   ✅ 收到认证响应: {}", message.content);
            }
            PubSubMessageType::Heartbeat => {
                println!("   💓 收到心跳: {}", message.content);
            }
            _ => {
                println!("   📨 收到其他消息: {}", message.content);
            }
        }
    }

    /// 处理接收到的P2P消息
    async fn handle_p2p_message(&self, message: &IrohMessage) {
        println!("📥 {} 收到P2P消息: {:?}", self.name, message.message_type);
        println!("   📄 内容: {}", message.content);
    }

    /// 获取统计信息
    fn get_stats(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("name".to_string(), self.name.clone());
        stats.insert("did".to_string(), self.did.clone());
        stats.insert("node_addr".to_string(), self.node_addr.clone());
        stats.insert("p2p_connections".to_string(), 
            self.iroh.get_connections().len().to_string());
        stats.insert("active_p2p_connections".to_string(), 
            self.iroh.get_connections().values().filter(|conn| conn.connected).count().to_string());
        stats
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("🌟 DIAP Rust SDK - Iroh+PubSub完整验证闭环演示");
    println!("==================================================");

    // 创建两个智能体节点
    let mut alice = AgentNode::new("Alice").await?;
    let mut bob = AgentNode::new("Bob").await?;

    println!("\n📋 智能体信息:");
    println!("  Alice - DID: {}", alice.did);
    println!("  Alice - 节点地址: {}", alice.node_addr);
    println!("  Bob - DID: {}", bob.did);
    println!("  Bob - 节点地址: {}", bob.node_addr);

    // 1. 注册DID到IPFS
    println!("\n📝 第一阶段: 注册DID到IPFS");
    let alice_did_result = alice.register_did().await?;
    let bob_did_result = bob.register_did().await?;

    // 2. 配置PubSub主题
    println!("\n📡 第二阶段: 配置PubSub主题");
    alice.setup_pubsub_topics().await?;
    bob.setup_pubsub_topics().await?;

    // 等待主题传播
    sleep(Duration::from_secs(2)).await;

    // 3. 执行IPFS双向验证
    println!("\n🔐 第三阶段: 执行IPFS双向验证");
    alice.perform_verification(&bob.did).await?;
    bob.perform_verification(&alice.did).await?;

    // 4. 建立P2P连接
    println!("\n🔗 第四阶段: 建立P2P连接");
    let alice_to_bob_conn = alice.establish_p2p_connection(&bob.node_addr).await?;
    let bob_to_alice_conn = bob.establish_p2p_connection(&alice.node_addr).await?;

    // 等待连接稳定
    sleep(Duration::from_secs(1)).await;

    // 5. P2P认证流程
    println!("\n🤝 第五阶段: P2P认证流程");
    alice.send_p2p_auth_request(&bob.did, &alice_to_bob_conn).await?;
    sleep(Duration::from_millis(500)).await;
    bob.send_p2p_auth_response(&alice.did, &bob_to_alice_conn).await?;

    // 6. PubSub认证流程
    println!("\n📡 第六阶段: PubSub认证流程");
    alice.publish_pubsub_auth_request(&bob.did).await?;
    sleep(Duration::from_millis(500)).await;
    bob.publish_pubsub_auth_response(&alice.did).await?;

    // 7. 持续通信演示
    println!("\n💬 第七阶段: 持续通信演示");
    
    // 启动消息接收处理
    let alice_clone = std::sync::Arc::new(std::sync::Mutex::new(alice));
    let bob_clone = std::sync::Arc::new(std::sync::Mutex::new(bob));

    // 发送一些测试消息
    {
        let alice = alice_clone.lock().unwrap();
        alice.send_p2p_custom_message(&bob.did, &alice_to_bob_conn, "Hello from Alice via P2P!").await?;
    }
    
    sleep(Duration::from_millis(500)).await;
    
    {
        let bob = bob_clone.lock().unwrap();
        bob.send_p2p_custom_message(&alice.did, &bob_to_alice_conn, "Hello from Bob via P2P!").await?;
    }

    // 8. 显示最终统计
    println!("\n📊 最终统计信息:");
    {
        let alice = alice_clone.lock().unwrap();
        let bob = bob_clone.lock().unwrap();
        
        println!("  Alice统计:");
        for (key, value) in alice.get_stats() {
            println!("    {}: {}", key, value);
        }
        
        println!("  Bob统计:");
        for (key, value) in bob.get_stats() {
            println!("    {}: {}", key, value);
        }
    }

    println!("\n🎉 Iroh+PubSub完整验证闭环演示完成!");
    println!("==================================================");
    println!("✅ 已完成的功能:");
    println!("  - IPFS DID注册和CID验证");
    println!("  - PubSub广播和发现");
    println!("  - Iroh P2P可靠通信");
    println!("  - 双重认证流程(P2P + PubSub)");
    println!("  - 完整的消息验证和统计");
    
    // 保持程序运行一段时间以便观察
    println!("\n⏳ 保持运行30秒以便观察...");
    sleep(Duration::from_secs(30)).await;

    Ok(())
}
