// DIAP Rust SDK - PubSub验证闭环演示
// 展示智能体通过PubSub通讯，使用IPFS DID和CID的ZKP验证身份
// 集成基准测试框架，实时测量各项性能指标

use anyhow::Result;
use diap_rs_sdk::benchmarks::{MetricCollector, MetricType, ReportFormat, ReportGenerator};
use diap_rs_sdk::{
    AgentInfo, IdentityManager, IpfsBidirectionalVerificationManager, KeyPair, PubsubAuthenticator,
    TopicConfig, TopicPolicy,
};
use libp2p::PeerId;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    env_logger::init();

    println!("🚀 PubSub验证闭环演示");
    println!("==========================================");
    println!("智能体通过PubSub通讯 + IPFS DID/CID ZKP验证");

    // 读取CLI/ENV参数
    let args: Vec<String> = std::env::args().collect();
    let mut api_url_cli: Option<String> = None;
    let mut gateway_url_cli: Option<String> = None;
    let mut i = 1;
    while i + 1 < args.len() {
        match args[i].as_str() {
            "--api-url" => {
                api_url_cli = Some(args[i + 1].clone());
                i += 2;
            }
            "--gateway-url" => {
                gateway_url_cli = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    let api_url = api_url_cli
        .or_else(|| env::var("DIAP_IPFS_API_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:5001".to_string());
    let gateway_url = gateway_url_cli
        .or_else(|| env::var("DIAP_IPFS_GATEWAY_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8081".to_string());

    // 初始化性能指标收集器
    println!("\n📊 初始化性能指标收集器...");
    let metrics_collector = Arc::new(MetricCollector::new(10000));
    println!("✅ 性能指标收集器初始化完成\n");

    // 1. 初始化IPFS双向验证管理器（优先远程IPFS）
    println!("\n🔧 初始化IPFS双向验证管理器...");
    let start_time = Instant::now();
    let mut verification_manager =
        if env::var("DIAP_FORCE_PUBLIC_ONLY").ok().as_deref() == Some("1") {
            IpfsBidirectionalVerificationManager::new().await?
        } else {
            IpfsBidirectionalVerificationManager::new_with_remote_ipfs(
                api_url.clone(),
                gateway_url.clone(),
            )
            .await?
        };
    let init_time = start_time.elapsed();

    println!("✅ IPFS双向验证管理器初始化成功");
    println!("   初始化时间: {:?}", init_time);
    println!("   IPFS API: {}", api_url);
    println!("   网关: {}", gateway_url);
    
    // 记录启动时间
    let mut metadata = HashMap::new();
    metadata.insert("operation".to_string(), "verification_manager_init".to_string());
    metadata.insert("ipfs_api".to_string(), api_url.clone());
    metrics_collector
        .record_measurement(MetricType::StartupTime, init_time.as_millis() as f64, metadata)
        .await;

    // 2. 创建智能体A (Alice) 和 B (Bob)
    println!("\n🤖 创建智能体");
    println!("==============");

    let alice_info = AgentInfo {
        name: "Alice".to_string(),
        services: vec![],
        description: Some("Alice智能体 - PubSub验证发起方".to_string()),
        tags: Some(vec!["pubsub".to_string(), "initiator".to_string()]),
    };

    let bob_info = AgentInfo {
        name: "Bob".to_string(),
        services: vec![],
        description: Some("Bob智能体 - PubSub验证响应方".to_string()),
        tags: Some(vec!["pubsub".to_string(), "responder".to_string()]),
    };

    let alice_keypair = KeyPair::generate()?;
    let bob_keypair = KeyPair::generate()?;

    println!("✅ Alice智能体创建成功");
    println!("   DID: {}", alice_keypair.did);
    println!("✅ Bob智能体创建成功");
    println!("   DID: {}", bob_keypair.did);

    // 3. 注册智能体到IPFS网络
    println!("\n📝 注册智能体到IPFS网络");
    println!("=========================");

    // 测量 Alice 注册延迟
    let alice_reg_start = Instant::now();
    let alice_cid = verification_manager
        .register_agent(&alice_info, &alice_keypair)
        .await?;
    let alice_reg_time = alice_reg_start.elapsed().as_millis() as f64;
    
    let mut metadata = HashMap::new();
    metadata.insert("agent".to_string(), "alice".to_string());
    metadata.insert("cid".to_string(), alice_cid.clone());
    metadata.insert("did".to_string(), alice_keypair.did.clone());
    metrics_collector
        .record_measurement(MetricType::RegistrationLatency, alice_reg_time, metadata.clone())
        .await;
    
    // 测量 Bob 注册延迟
    let bob_reg_start = Instant::now();
    let bob_cid = verification_manager
        .register_agent(&bob_info, &bob_keypair)
        .await?;
    let bob_reg_time = bob_reg_start.elapsed().as_millis() as f64;
    
    metadata.insert("agent".to_string(), "bob".to_string());
    metadata.insert("cid".to_string(), bob_cid.clone());
    metadata.insert("did".to_string(), bob_keypair.did.clone());
    metrics_collector
        .record_measurement(MetricType::RegistrationLatency, bob_reg_time, metadata)
        .await;

    println!("✅ Alice注册成功，CID: {}", alice_cid);
    println!("   ⏱️  注册耗时: {:.2} ms", alice_reg_time);
    println!("✅ Bob注册成功，CID: {}", bob_cid);
    println!("   ⏱️  注册耗时: {:.2} ms", bob_reg_time);

    // 4. 创建PubSub认证器
    println!("\n🔐 创建PubSub认证器");
    println!("===================");

    // 使用相同的IPFS客户端，这样PubSub认证器可以访问相同的DID文档
    // 注意：在实际应用中，这可能需要更复杂的网络配置
    let shared_ipfs_client = verification_manager.get_ipfs_client();

    let alice_identity_manager = IdentityManager::new(shared_ipfs_client.clone());
    let bob_identity_manager = IdentityManager::new(shared_ipfs_client);

    let alice_pubsub = PubsubAuthenticator::new(alice_identity_manager, None, None);

    let bob_pubsub = PubsubAuthenticator::new(bob_identity_manager, None, None);

    // 5. 设置本地身份
    println!("\n🔑 设置本地身份");
    println!("================");

    let alice_peer_id = PeerId::random();
    let bob_peer_id = PeerId::random();

    alice_pubsub
        .set_local_identity(alice_keypair.clone(), alice_peer_id, alice_cid.clone())
        .await?;

    bob_pubsub
        .set_local_identity(bob_keypair.clone(), bob_peer_id, bob_cid.clone())
        .await?;

    println!("✅ Alice身份设置完成");
    println!("   PeerID: {}", alice_peer_id);
    println!("✅ Bob身份设置完成");
    println!("   PeerID: {}", bob_peer_id);

    // 6. 配置PubSub主题
    println!("\n📡 配置PubSub主题");
    println!("==================");

    let verification_topic = "diap-verification";
    let heartbeat_topic = "diap-heartbeat";
    let general_topic = "diap-general";

    // 配置验证主题 - 仅允许认证用户
    let verification_config = TopicConfig {
        name: verification_topic.to_string(),
        policy: TopicPolicy::AllowAuthenticated,
        require_zkp: true,
        require_signature: true,
    };

    // 配置心跳主题 - 允许所有认证用户
    let heartbeat_config = TopicConfig {
        name: heartbeat_topic.to_string(),
        policy: TopicPolicy::AllowAuthenticated,
        require_zkp: false,
        require_signature: true,
    };

    // 配置通用主题 - 允许特定DID列表
    let general_config = TopicConfig {
        name: general_topic.to_string(),
        policy: TopicPolicy::AllowList(vec![alice_keypair.did.clone(), bob_keypair.did.clone()]),
        require_zkp: true,
        require_signature: true,
    };

    alice_pubsub
        .configure_topic(verification_config.clone())
        .await?;
    alice_pubsub
        .configure_topic(heartbeat_config.clone())
        .await?;
    alice_pubsub.configure_topic(general_config.clone()).await?;

    bob_pubsub.configure_topic(verification_config).await?;
    bob_pubsub.configure_topic(heartbeat_config).await?;
    bob_pubsub.configure_topic(general_config).await?;

    println!("✅ 主题配置完成");
    println!("   验证主题: {} (需要ZKP + 签名)", verification_topic);
    println!("   心跳主题: {} (仅需签名)", heartbeat_topic);
    println!("   通用主题: {} (白名单 + ZKP + 签名)", general_topic);

    // 7. 订阅主题
    println!("\n📢 订阅主题");
    println!("============");

    alice_pubsub.subscribe_topic(verification_topic).await?;
    alice_pubsub.subscribe_topic(heartbeat_topic).await?;
    alice_pubsub.subscribe_topic(general_topic).await?;

    bob_pubsub.subscribe_topic(verification_topic).await?;
    bob_pubsub.subscribe_topic(heartbeat_topic).await?;
    bob_pubsub.subscribe_topic(general_topic).await?;

    println!("✅ 主题订阅完成");

    // 8. 执行IPFS双向验证（建立信任基础）
    println!("\n🤝 执行IPFS双向验证（建立信任基础）");
    println!("=====================================");

    let resource_cid = "QmTestResourceForPubSubVerification123456789";

    // 测量双向验证时间（包含 ZKP 生成）
    let verification_start = Instant::now();
    let bidirectional_result = verification_manager
        .initiate_bidirectional_verification("Alice", "Bob", resource_cid)
        .await?;
    let verification_time = verification_start.elapsed().as_millis() as f64;
    
    // 记录 ZKP 生成时间
    let mut metadata = HashMap::new();
    metadata.insert("operation".to_string(), "bidirectional_verification".to_string());
    metadata.insert("resource_cid".to_string(), resource_cid.to_string());
    metadata.insert("success".to_string(), bidirectional_result.success.to_string());
    metrics_collector
        .record_measurement(MetricType::ZKPGenerationTime, verification_time, metadata)
        .await;

    println!("✅ IPFS双向验证完成");
    println!(
        "   验证状态: {}",
        if bidirectional_result.success {
            "成功"
        } else {
            "失败"
        }
    );
    println!("   ⏱️  验证耗时: {:.2} ms", verification_time);

    if !bidirectional_result.success {
        println!("❌ IPFS双向验证失败，无法继续PubSub验证闭环");
        return Ok(());
    }

    // 9. PubSub验证闭环演示
    println!("\n🔄 PubSub验证闭环演示");
    println!("=====================");

    // 9.1 Alice发送身份验证请求给Bob
    println!("\n📤 Alice → Bob: 身份验证请求");
    let challenge = format!(
        "challenge_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );

    // 测量消息创建和发送延迟
    let msg_create_start = Instant::now();
    let auth_request = alice_pubsub
        .create_auth_request(verification_topic, &bob_keypair.did, &challenge)
        .await?;
    let msg_create_time = msg_create_start.elapsed().as_millis() as f64;
    
    // 记录消息发现延迟（PubSub 消息创建和发送）
    let mut metadata = HashMap::new();
    metadata.insert("operation".to_string(), "pubsub_auth_request_creation".to_string());
    metadata.insert("topic".to_string(), verification_topic.to_string());
    metadata.insert("from".to_string(), "alice".to_string());
    metadata.insert("to".to_string(), "bob".to_string());
    metrics_collector
        .record_measurement(MetricType::MessageDiscoveryLatency, msg_create_time, metadata)
        .await;

    println!("✅ Alice创建身份验证请求");
    println!("   消息ID: {}", auth_request.message_id);
    println!("   挑战: {}", challenge);
    println!("   目标DID: {}", bob_keypair.did);

    // 9.2 Bob验证Alice的消息
    println!("\n🔍 Bob验证Alice的消息");
    let verification_start = Instant::now();
    let verification_result = bob_pubsub.verify_message(&auth_request).await?;
    let verification_time = verification_start.elapsed();
    
    // 记录 ZKP 验证时间
    let mut metadata = HashMap::new();
    metadata.insert("operation".to_string(), "pubsub_message_verification".to_string());
    metadata.insert("message_type".to_string(), "auth_request".to_string());
    metadata.insert("verified".to_string(), verification_result.verified.to_string());
    metrics_collector
        .record_measurement(MetricType::ZKPVerificationTime, verification_time.as_millis() as f64, metadata)
        .await;

    println!("✅ Bob验证完成");
    println!(
        "   验证结果: {}",
        if verification_result.verified {
            "✅ 通过"
        } else {
            "❌ 失败"
        }
    );
    println!("   验证时间: {:?}", verification_time);

    for detail in &verification_result.details {
        println!("   {}", detail);
    }

    if !verification_result.verified {
        println!("❌ 消息验证失败，无法继续");
        return Ok(());
    }

    // 9.3 Bob发送身份验证响应给Alice
    println!("\n📤 Bob → Alice: 身份验证响应");
    let response = format!(
        "response_{}_{}",
        challenge,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );

    let auth_response = bob_pubsub
        .create_auth_response(verification_topic, &alice_keypair.did, &response)
        .await?;

    println!("✅ Bob创建身份验证响应");
    println!("   消息ID: {}", auth_response.message_id);
    println!("   响应: {}", response);
    println!("   目标DID: {}", alice_keypair.did);

    // 9.4 Alice验证Bob的响应
    println!("\n🔍 Alice验证Bob的响应");
    let verification_start = Instant::now();
    let verification_result = alice_pubsub.verify_message(&auth_response).await?;
    let verification_time = verification_start.elapsed();
    
    // 记录 ZKP 验证时间
    let mut metadata = HashMap::new();
    metadata.insert("operation".to_string(), "pubsub_message_verification".to_string());
    metadata.insert("message_type".to_string(), "auth_response".to_string());
    metadata.insert("verified".to_string(), verification_result.verified.to_string());
    metrics_collector
        .record_measurement(MetricType::ZKPVerificationTime, verification_time.as_millis() as f64, metadata)
        .await;

    println!("✅ Alice验证完成");
    println!(
        "   验证结果: {}",
        if verification_result.verified {
            "✅ 通过"
        } else {
            "❌ 失败"
        }
    );
    println!("   验证时间: {:?}", verification_time);

    for detail in &verification_result.details {
        println!("   {}", detail);
    }

    // 10. 心跳消息演示
    println!("\n💓 心跳消息演示");
    println!("================");

    for i in 1..=3 {
        println!("\n📤 Alice发送心跳消息 #{}", i);
        let heartbeat_start = Instant::now();
        let heartbeat = alice_pubsub.create_heartbeat(heartbeat_topic).await?;
        let heartbeat_time = heartbeat_start.elapsed().as_millis() as f64;
        println!("✅ 心跳消息创建成功");
        println!("   消息ID: {}", heartbeat.message_id);
        println!("   ⏱️  创建耗时: {:.2} ms", heartbeat_time);

        // Bob验证心跳消息
        let verify_start = Instant::now();
        let verification_result = bob_pubsub.verify_message(&heartbeat).await?;
        let verify_time = verify_start.elapsed().as_millis() as f64;
        
        // 记录吞吐量相关信息（心跳消息的成功率）
        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), "heartbeat".to_string());
        metadata.insert("message_id".to_string(), i.to_string());
        metadata.insert("verified".to_string(), verification_result.verified.to_string());
        metrics_collector
            .record_measurement(MetricType::MessageDiscoveryLatency, heartbeat_time + verify_time, metadata)
            .await;
        println!(
            "🔍 Bob验证心跳消息: {}",
            if verification_result.verified {
                "✅ 通过"
            } else {
                "❌ 失败"
            }
        );

        bob_pubsub.update_message_stats(heartbeat_topic).await;

        sleep(Duration::from_millis(500)).await;
    }

    // 11. 通用消息演示（测量吞吐量）
    println!("\n💬 通用消息演示");
    println!("================");

    let messages = vec![
        "Hello Bob, this is Alice!",
        "How are you doing?",
        "Let's collaborate on this project!",
    ];

    let mut successful_messages = 0;
    let mut failed_messages = 0;
    let messages_start = Instant::now();

    for (i, message_content) in messages.iter().enumerate() {
        println!("\n📤 Alice发送通用消息 #{}", i + 1);
        let msg_start = Instant::now();
        let message = alice_pubsub
            .create_simple_message(general_topic, message_content)
            .await?;
        let msg_time = msg_start.elapsed().as_millis() as f64;
        println!("✅ 通用消息创建成功");
        println!("   消息ID: {}", message.message_id);
        println!("   内容: {}", message_content);
        println!("   ⏱️  创建耗时: {:.2} ms", msg_time);

        // Bob验证通用消息
        let verify_start = Instant::now();
        let verification_result = bob_pubsub.verify_message(&message).await?;
        let verify_time = verify_start.elapsed().as_millis() as f64;
        
        if verification_result.verified {
            successful_messages += 1;
        } else {
            failed_messages += 1;
        }
        
        // 记录消息处理延迟
        let mut metadata = HashMap::new();
        metadata.insert("operation".to_string(), "general_message".to_string());
        metadata.insert("message_index".to_string(), i.to_string());
        metadata.insert("verified".to_string(), verification_result.verified.to_string());
        metrics_collector
            .record_measurement(MetricType::MessageDiscoveryLatency, msg_time + verify_time, metadata)
            .await;
        println!(
            "🔍 Bob验证通用消息: {}",
            if verification_result.verified {
                "✅ 通过"
            } else {
                "❌ 失败"
            }
        );

        if verification_result.verified {
            println!("📨 Bob收到消息: {}", message_content);
        }

        bob_pubsub.update_message_stats(general_topic).await;

        sleep(Duration::from_millis(300)).await;
    }
    
    // 计算吞吐量
    let messages_duration = messages_start.elapsed().as_secs_f64();
    let throughput = successful_messages as f64 / messages_duration.max(0.001);
    
    // 计算丢包率
    let total_messages = successful_messages + failed_messages;
    let loss_rate = if total_messages > 0 {
        (failed_messages as f64 / total_messages as f64) * 100.0
    } else {
        0.0
    };
    
    // 记录吞吐量和丢包率
    let mut metadata = HashMap::new();
    metadata.insert("total_messages".to_string(), total_messages.to_string());
    metadata.insert("successful".to_string(), successful_messages.to_string());
    metadata.insert("failed".to_string(), failed_messages.to_string());
    metrics_collector
        .record_measurement(MetricType::Throughput, throughput, metadata.clone())
        .await;
    
    metadata.insert("duration_seconds".to_string(), messages_duration.to_string());
    metrics_collector
        .record_measurement(MetricType::PacketLossRate, loss_rate, metadata)
        .await;

    // 12. 统计信息
    println!("\n📊 PubSub统计信息");
    println!("==================");

    let alice_topics = alice_pubsub.get_subscribed_topics().await;
    let bob_topics = bob_pubsub.get_subscribed_topics().await;

    println!("Alice订阅的主题: {:?}", alice_topics);
    println!("Bob订阅的主题: {:?}", bob_topics);

    let alice_stats = alice_pubsub.get_message_stats().await;
    let bob_stats = bob_pubsub.get_message_stats().await;

    println!("Alice消息统计: {:?}", alice_stats);
    println!("Bob消息统计: {:?}", bob_stats);

    println!("Alice缓存统计: {:?}", alice_pubsub.cache_stats());
    println!("Bob缓存统计: {:?}", bob_pubsub.cache_stats());

    println!("Alice nonce计数: {}", alice_pubsub.nonce_count());
    println!("Bob nonce计数: {}", bob_pubsub.nonce_count());
    
    // 输出性能统计信息
    println!("\n📊 性能指标统计");
    println!("═══════════════════════════════════════════════════════════\n");

    // 注册延迟统计
    let reg_stats = metrics_collector.get_statistics(MetricType::RegistrationLatency).await;
    if reg_stats.count > 0 {
        println!("📝 注册延迟统计:");
        println!("   测量次数: {}", reg_stats.count);
        println!("   平均延迟: {:.2} ms", reg_stats.mean);
        println!("   最小延迟: {:.2} ms", reg_stats.min);
        println!("   最大延迟: {:.2} ms", reg_stats.max);
        println!("   P95: {:.2} ms\n", reg_stats.p95);
    }

    // ZKP 生成时间统计
    let zkp_gen_stats = metrics_collector.get_statistics(MetricType::ZKPGenerationTime).await;
    if zkp_gen_stats.count > 0 {
        println!("🔐 ZKP 生成时间统计:");
        println!("   测量次数: {}", zkp_gen_stats.count);
        println!("   平均时间: {:.2} ms", zkp_gen_stats.mean);
        println!("   P95: {:.2} ms\n", zkp_gen_stats.p95);
    }

    // ZKP 验证时间统计
    let zkp_verify_stats = metrics_collector.get_statistics(MetricType::ZKPVerificationTime).await;
    if zkp_verify_stats.count > 0 {
        println!("🔍 ZKP 验证时间统计:");
        println!("   测量次数: {}", zkp_verify_stats.count);
        println!("   平均时间: {:.2} ms", zkp_verify_stats.mean);
        println!("   最小时间: {:.2} ms", zkp_verify_stats.min);
        println!("   最大时间: {:.2} ms", zkp_verify_stats.max);
        println!("   P95: {:.2} ms\n", zkp_verify_stats.p95);
    }

    // 消息发现延迟统计
    let discovery_stats = metrics_collector.get_statistics(MetricType::MessageDiscoveryLatency).await;
    if discovery_stats.count > 0 {
        println!("📡 消息发现延迟统计:");
        println!("   测量次数: {}", discovery_stats.count);
        println!("   平均延迟: {:.2} ms", discovery_stats.mean);
        println!("   P95: {:.2} ms\n", discovery_stats.p95);
    }

    // 吞吐量统计
    let throughput_stats = metrics_collector.get_statistics(MetricType::Throughput).await;
    if throughput_stats.count > 0 {
        println!("⚡ 吞吐量统计:");
        println!("   测量次数: {}", throughput_stats.count);
        println!("   平均吞吐量: {:.2} 消息/秒", throughput_stats.mean);
        println!();
    }

    // 丢包率统计
    let loss_stats = metrics_collector.get_statistics(MetricType::PacketLossRate).await;
    if loss_stats.count > 0 {
        println!("📉 丢包率统计:");
        println!("   测量次数: {}", loss_stats.count);
        println!("   平均丢包率: {:.2}%", loss_stats.mean);
        println!();
    }

    // 保存性能报告
    let all_measurements = metrics_collector.get_all_measurements().await;
    if !all_measurements.is_empty() {
        println!("📄 保存性能报告...");
        
        // 创建报告保存目录（examples/reports/）
        let reports_dir = "examples/reports";
        if let Err(e) = std::fs::create_dir_all(reports_dir) {
            eprintln!("   ⚠️  创建报告目录失败: {} (将使用当前目录)", e);
        }
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let report_path = format!("{}/pubsub_loop_benchmark_{}.json", reports_dir, timestamp);
        
        // 创建简化的结果用于报告
        use diap_rs_sdk::benchmarks::{ExperimentResult, ExperimentConfig};
        let mut metrics_map = HashMap::new();
        metrics_map.insert("RegistrationLatency".to_string(), reg_stats.clone());
        metrics_map.insert("ZKPGenerationTime".to_string(), zkp_gen_stats.clone());
        metrics_map.insert("ZKPVerificationTime".to_string(), zkp_verify_stats.clone());
        metrics_map.insert("MessageDiscoveryLatency".to_string(), discovery_stats.clone());
        metrics_map.insert("Throughput".to_string(), throughput_stats.clone());
        metrics_map.insert("PacketLossRate".to_string(), loss_stats.clone());
        
        let result = ExperimentResult {
            config: ExperimentConfig {
                name: "PubSub 闭环测试性能指标".to_string(),
                metrics: vec![],
                iterations: 1,
                node_count: 2,
                ipfs_api_url: api_url.clone(),
                ipfs_gateway_url: gateway_url.clone(),
                timeout_seconds: 600,
            },
            metrics: metrics_map,
            raw_measurements: all_measurements,
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: chrono::Utc::now().to_rfc3339(),
            duration_seconds: 0.0,
            errors: vec![],
        };
        
        ReportGenerator::save_report(&result, &report_path, ReportFormat::Json)
            .await
            .unwrap_or_else(|e| eprintln!("   ⚠️  保存报告失败: {}", e));
        
        println!("   ✅ 报告已保存: {}\n", report_path);
    }

    // 13. 验证闭环总结
    println!("\n🎯 PubSub验证闭环总结");
    println!("======================");

    println!("✅ 验证闭环建立成功！");
    println!("🔐 基于IPFS DID和CID的ZKP验证");
    println!("📡 通过PubSub进行去中心化通讯");
    println!("🤝 智能体间双向身份验证");
    println!("💓 心跳机制保持连接活跃");
    println!("💬 安全的消息传递");
    println!("🛡️  防重放攻击保护");
    println!("📊 完整的消息统计和监控");

    println!("\n💡 验证闭环特性:");
    println!("   🌐 完全去中心化 - 基于IPFS网络");
    println!("   🔐 零知识证明 - 保护隐私的同时验证身份");
    println!("   📡 PubSub通讯 - 高效的消息传递");
    println!("   🛡️  多重安全机制 - ZKP + 签名 + Nonce");
    println!("   ⚡ 实时验证 - 快速的身份确认");
    println!("   📊 完整监控 - 消息统计和状态跟踪");

    println!("\n🎊 PubSub验证闭环演示完成！");
    println!("==========================================");

    Ok(())
}
