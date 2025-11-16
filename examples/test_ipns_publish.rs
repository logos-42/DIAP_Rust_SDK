use anyhow::Result;
/**
 * IPNS 发布功能测试示例
 * 演示如何使用 IpfsClient 的 IPNS 发布功能
 */
use diap_rs_sdk::IpfsClient;
// 追加用于真实 PubSub + ZKP 验证所需类型
use diap_rs_sdk::{
    did_builder::DIDBuilder,
    identity_manager::IdentityManager,
    key_manager::KeyPair,
    pubsub_authenticator::{TopicConfig, TopicPolicy},
};
use libp2p::PeerId;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🚀 IPNS 发布功能测试");
    println!("{}", "=".repeat(50));

    // 1. 创建 IPFS 客户端（连接本地 Kubo）
    let api_url =
        std::env::var("DIAP_IPFS_API_URL").unwrap_or_else(|_| "http://127.0.0.1:5001".to_string());
    let gateway_url = std::env::var("DIAP_IPFS_GATEWAY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    println!("\n📡 连接到 IPFS 节点:");
    println!("   API: {}", api_url);
    println!("   网关: {}", gateway_url);

    let ipfs_client = IpfsClient::new_with_remote_node(api_url.clone(), gateway_url.clone(), 30);

    // 2. 上传测试内容到 IPFS
    println!("\n📤 上传测试内容到 IPFS...");
    // 展示真实 DID 格式（示例 did:key，演示用途）
    let example_did = "did:key:z6MkqYgH4b7yR3y3q7Qf2NV7wQYxkZC9p7kC4k9wYQpX1A2B";
    let test_content = serde_json::json!({
        "test": "ipns_publish_test",
        "did": example_did,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "这是一个 IPNS 发布测试"
    });

    let upload_result = ipfs_client
        .upload(
            &serde_json::to_string_pretty(&test_content)?,
            "test_ipns.json",
        )
        .await?;

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
    let lifetime = "24h"; // 24小时
    let ttl = "1h"; // 1小时

    match ipfs_client
        .publish_ipns(&upload_result.cid, key_name, lifetime, ttl)
        .await
    {
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
            // 更新内容同样包含 DID，便于观察完整链路
            let new_content = serde_json::json!({
                "test": "updated_content",
                "did": example_did,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": "这是更新后的内容"
            });

            let new_upload = ipfs_client
                .upload(
                    &serde_json::to_string_pretty(&new_content)?,
                    "test_ipns_updated.json",
                )
                .await?;

            println!("   新 CID: {}", new_upload.cid);

            match ipfs_client
                .publish_after_upload(&new_upload.cid, key_name, lifetime, ttl)
                .await
            {
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

            // 7. 演示：ZKP 通过 PubSub 载荷的编码/解码流程（最小可运行，不做网络发送/验证）
            println!("\n🧪 演示：ZKP 使用 PubSub 解码流程（最小示例）");
            use diap_rs_sdk::pubsub_authenticator::{AuthenticatedMessage, PubSubMessageType, PubsubAuthenticator};
            // 构造一个带有 IPNS did_cid 与模拟 zkp_proof 的消息，并进行序列化/反序列化演示
            let ipns_name = format!("/ipns/{}", ipns_result.name);
            let sample_msg = AuthenticatedMessage {
                message_id: uuid::Uuid::new_v4().to_string(),
                message_type: PubSubMessageType::AuthRequest,
                from_did: example_did.to_string(),
                to_did: None,
                from_peer_id: "12D3KooWExamplePeerIdForDemoOnly".to_string(),
                did_cid: ipns_name.clone(), // 关键：这里使用 IPNS 名称
                topic: "diap-demo".to_string(),
                content: br#"{"note":"demo pubsub payload"}"#.to_vec(),
                nonce: "demo-nonce-123".to_string(),
                zkp_proof: vec![1, 2, 3, 4], // 模拟的 ZKP 载荷字节
                signature: vec![0; 64],      // 演示用占位符
                timestamp: (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap())
                .as_secs(),
            };

            // 编码为可以在 PubSub 中传输的字节
            let encoded = PubsubAuthenticator::serialize_message(&sample_msg)?;
            println!("   ✓ 已编码 PubSub 认证消息，长度: {} bytes", encoded.len());

            // 在接收端解码字节为结构体
            let decoded = PubsubAuthenticator::deserialize_message(&encoded)?;
            println!("   ✓ 已解码 PubSub 消息");
            println!("     - message_id: {}", decoded.message_id);
            println!("     - from_did  : {}", decoded.from_did);
            println!("     - did_cid   : {}", decoded.did_cid);
            println!("     - zkp_proof : {} bytes", decoded.zkp_proof.len());
            println!("     - 说明      : 在实际 verify_message 流程中，如果 did_cid 是 IPNS，SDK 会先解析为 CID，再拉取 DID 文档并进行 ZKP 验证");

            // 8. 真实演示：构建 DID 文档 → 将 IPNS 指向 DID CID → PubSub 认证消息 → 自动解析 IPNS 并进行 ZKP + 签名验证
            println!("\n🔒 真实演示：PubSub + ZKP 验证（使用 IPNS did_cid）");

            // 8.1 生成密钥与 PeerId，创建并发布 DID 文档
            let keypair = KeyPair::generate()?;
            let peer_id = PeerId::random();
            let did_builder = DIDBuilder::new(ipfs_client.clone());
            let did_pub = did_builder.create_and_publish(&keypair, &peer_id).await?;
            println!("   ✓ DID 已发布");
            println!("     - DID: {}", did_pub.did);
            println!("     - DID CID: {}", did_pub.cid);

            // 8.2 将 IPNS 名称指向 DID CID（这样验证时 IPNS→CID 会得到 DID 文档）
            let updated = ipfs_client
                .publish_ipns(&did_pub.cid, key_name, lifetime, ttl)
                .await?;
            let did_ipns = format!("/ipns/{}", updated.name);
            println!("   ✓ IPNS 指向 DID");
            println!("     - IPNS: {}", did_ipns);
            println!("     - Path: {}", updated.value);

            // 8.3 初始化认证器并配置主题
            let idm = IdentityManager::new(ipfs_client.clone());
            let auth = PubsubAuthenticator::new(idm, None, None);
            auth.set_local_identity(keypair.clone(), peer_id, did_pub.cid.clone()).await?;
            // 从 DID 文档抽取 pubsub auth 主题（或回退到默认）
            let auth_topic = diap_rs_sdk::pubsub_authenticator::PubsubAuthenticator::extract_auth_topic_from_did(&did_pub.did_document)
                .unwrap_or_else(|| "diap-auth-default".to_string());
            auth.configure_topic(TopicConfig {
                name: auth_topic.clone(),
                policy: TopicPolicy::AllowAuthenticated,
                require_zkp: true,
                require_signature: true,
            }).await?;
            println!("   ✓ 配置 PubSub 认证主题: {}", auth_topic);

            // 8.4 创建一条认证请求消息，并将 did_cid 替换为 IPNS 名称以触发 IPNS→CID→ZKP 验证路径
            let challenge = format!("challenge-{}", chrono::Utc::now().timestamp());
            let req = auth.create_auth_request(&auth_topic, &keypair.did, &challenge).await?;
            let mut req_ipns = req.clone();
            req_ipns.did_cid = did_ipns.clone();

            // 8.5 演示“发送/接收”：序列化后立刻反序列化（模拟网络传输）
            let network_bytes = PubsubAuthenticator::serialize_message(&req_ipns)?;
            let received = PubsubAuthenticator::deserialize_message(&network_bytes)?;

            // 8.6 验证消息：这一步会自动解析 IPNS → CID，拉取 DID 文档并进行 ZKP + 签名验证
            let verify = auth.verify_message(&received).await?;
            println!("   ✓ 验证完成: {}", if verify.verified { "通过" } else { "失败" });
            for line in &verify.details {
                println!("     - {}", line);
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
