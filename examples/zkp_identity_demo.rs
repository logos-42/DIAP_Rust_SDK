// DIAP Rust SDK - ZKP身份绑定演示
// 展示使用ZKP验证DID-CID绑定的完整流程

use diap_rs_sdk::*;
use anyhow::Result;
use libp2p::identity::Keypair as LibP2PKeypair;
use libp2p::PeerId;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("\n🔐 DIAP ZKP身份绑定演示\n");
    println!("========================================");
    println!("架构说明：");
    println!("  - 使用 did:key 格式（无需IPNS）");
    println!("  - DID私钥加密PeerID");
    println!("  - ZKP证明DID-CID绑定关系");
    println!("  - 单次IPFS上传，简化流程");
    println!("========================================\n");
    
    // ==================== 第1步：初始化 ====================
    println!("📦 第1步：初始化组件");
    println!("  注意：此演示需要先生成ZKP keys");
    println!("  运行: cargo run --example zkp_setup_keys");
    println!();
    
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None, None, 30,
    );
    
    // 尝试加载ZKP keys
    let identity_manager = match IdentityManager::new_with_keys(
        ipfs_client.clone(),
        "zkp_proving.key",
        "zkp_verifying.key",
    ) {
        Ok(manager) => {
            println!("✓ IPFS客户端已连接");
            println!("✓ ZKP keys已加载");
            println!("✓ 身份管理器已创建（使用真实Groth16 ZKP）\n");
            manager
        }
        Err(e) => {
            eprintln!("❌ 无法加载ZKP keys: {}", e);
            eprintln!();
            eprintln!("请先生成ZKP keys:");
            eprintln!("  cargo run --example zkp_setup_keys");
            eprintln!();
            return Err(e);
        }
    };
    
    // ==================== 第2步：生成密钥 ====================
    println!("🔑 第2步：生成密钥对");
    
    // DID密钥对
    let keypair = KeyPair::generate()?;
    println!("✓ DID密钥生成完成");
    println!("  DID: {}", keypair.did);
    println!("  类型: did:key (Ed25519)");
    
    // 验证DID格式
    assert!(keypair.did.starts_with("did:key:z"), "DID格式应为 did:key:z...");
    println!("  ✓ DID格式验证通过");
    
    // libp2p PeerID
    let libp2p_keypair = LibP2PKeypair::generate_ed25519();
    let peer_id = PeerId::from(libp2p_keypair.public());
    println!("✓ libp2p PeerID生成完成");
    println!("  PeerID: {}\n", peer_id);
    
    // ==================== 第3步：注册身份 ====================
    println!("📝 第3步：注册身份到IPFS");
    
    let agent_info = AgentInfo {
        name: "ZKP演示智能体".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "API".to_string(),
                endpoint: serde_json::json!({
                    "url": "https://api.example.com",
                    "version": "1.0"
                }),
            },
        ],
        description: Some("展示ZKP身份绑定的演示智能体".to_string()),
        tags: Some(vec!["zkp".to_string(), "demo".to_string()]),
    };
    
    let registration = identity_manager
        .register_identity(&agent_info, &keypair, &peer_id)
        .await?;
    
    println!("✅ 身份注册成功！");
    println!("  DID: {}", registration.did);
    println!("  CID: {}", registration.cid);
    println!("  加密PeerID: {}...", &registration.encrypted_peer_id_hex[..16]);
    println!();
    
    // ==================== 第4步：查看DID文档 ====================
    println!("📄 第4步：查看DID文档结构");
    
    let did_doc_json = serde_json::to_string_pretty(&registration.did_document)?;
    println!("{}", did_doc_json);
    println!();
    
    // ==================== 第5步：生成ZKP证明 ====================
    println!("🔐 第5步：生成DID-CID绑定证明（Groth16）");
    
    let nonce = b"challenge_nonce_from_resource_node_12345";
    println!("  挑战nonce: {:?}", String::from_utf8_lossy(nonce));
    
    let proof = identity_manager.generate_binding_proof(
        &keypair,
        &registration.did_document,
        &registration.cid,
        nonce,
    )?;
    
    println!("✅ ZKP证明生成成功");
    println!("  证明大小: {} 字节", proof.proof.len());
    println!("  生成时间: {}", proof.timestamp);
    println!("  公共输入数量: {}", proof.public_inputs.len());
    println!();
    
    // ==================== 第6步：验证身份 ====================
    println!("🔍 第6步：验证身份（模拟资源节点）");
    println!("  资源节点只知道: CID + ZKP证明 + nonce");
    println!("  资源节点不知道: 私钥、真实PeerID");
    println!();
    
    let verification = identity_manager.verify_identity_with_zkp(
        &registration.cid,
        &proof.proof,
        nonce,
    ).await?;
    
    println!("📊 验证结果：");
    println!("  DID: {}", verification.did);
    println!("  CID: {}", verification.cid);
    println!("  ZKP验证: {}", if verification.zkp_verified { "✅ 通过" } else { "❌ 失败" });
    println!();
    
    println!("验证详情：");
    for detail in &verification.verification_details {
        println!("  {}", detail);
    }
    println!();
    
    // ==================== 第7步：解密PeerID ====================
    println!("🔓 第7步：解密PeerID（需要私钥）");
    
    let encrypted_peer_id = identity_manager.extract_encrypted_peer_id(&registration.did_document)?;
    let decrypted_peer_id = identity_manager.decrypt_peer_id(&keypair, &encrypted_peer_id)?;
    
    println!("✓ PeerID解密成功");
    println!("  原始PeerID: {}", peer_id);
    println!("  解密PeerID: {}", decrypted_peer_id);
    println!("  匹配: {}", peer_id == decrypted_peer_id);
    println!();
    
    // ==================== 总结 ====================
    println!("========================================");
    println!("✅ 演示完成！");
    println!();
    println!("关键特性：");
    println!("  ✓ 使用 did:key 标准格式");
   
    println!("  ✓ DID-CID通过Groth16 ZKP强绑定");
    println!("  ✓ PeerID加密保护隐私");
    println!("  ✓ 完全去中心化验证");
    println!();
    println!("安全保障：");
    println!("  • 零知识证明：使用Groth16 SNARK");
    println!("  • 哈希绑定：H(DID文档) == CID");
    println!("  • 密钥证明：证明持有私钥（无需暴露）");
    println!("  • 加密PeerID：AES-256-GCM加密");
    println!("  • 防重放：每次使用新nonce");
    println!("========================================\n");
    
    Ok(())
}

