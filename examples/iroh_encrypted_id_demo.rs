use anyhow::Result;
use diap_rs_sdk::{did_builder::DIDBuilder, identity_manager::IdentityManager, key_manager::KeyPair, IpfsClient};
use iroh::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // 1) IPFS 客户端
    let api_url = std::env::var("DIAP_IPFS_API_URL").unwrap_or_else(|_| "http://127.0.0.1:5001".to_string());
    let gateway_url = std::env::var("DIAP_IPFS_GATEWAY_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let ipfs = IpfsClient::new_with_remote_node(api_url.clone(), gateway_url.clone(), 30);

    println!("API: {}, Gateway: {}", api_url, gateway_url);

    // 2) 启动真实 Iroh 端点并获取 node_id 作为明文
    let ep = Endpoint::builder().bind().await?;
    let node_addr = ep.node_addr();
    let node_id_str = node_addr.node_id.to_string();
    let iroh_node_id_plain = node_id_str.as_bytes().to_vec();
    println!("Iroh node_id: {}", node_id_str);

    // 3) 生成 DID，并把真实 Iroh ID 加密写入 DID 文档
    let keypair = KeyPair::generate()?;
    let mut builder = DIDBuilder::new(ipfs.clone());
    builder.set_iroh_node_id(&iroh_node_id_plain);

    // libp2p PeerId 可使用随机
    let peer_id = libp2p::PeerId::random();
    let publish = builder.create_and_publish(&keypair, &peer_id).await?;

    println!("DID: {}", publish.did);
    println!("DID CID: {}", publish.cid);

    // 4) 读取 DID，提取并解密 Iroh ID，校验与本地端一致
    let idm = IdentityManager::new(ipfs.clone());
    let enc = idm.extract_encrypted_iroh_id(&publish.did_document)?;
    let decrypted = idm.decrypt_iroh_id(&keypair, &enc)?;
    let decrypted_str = String::from_utf8_lossy(&decrypted);

    println!("Decrypted Iroh ID: {}", decrypted_str);
    let ok = decrypted == iroh_node_id_plain;
    println!("Match with local Iroh node_id: {}", if ok { "YES" } else { "NO" });

    Ok(())
}

