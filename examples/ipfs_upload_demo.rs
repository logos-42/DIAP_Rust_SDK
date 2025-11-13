use diap_rs_sdk::{IpfsClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 使用本地 Kubo 配置：API 5001, 网关 8081（与用户提供的配置一致）
    let client = IpfsClient::new_with_remote_node(
        "http://127.0.0.1:5001".to_string(),
        "http://127.0.0.1:8081".to_string(),
        60,
    );

    // 待上传的 JSON 内容
    let payload = serde_json::json!({
        "hello": "ipfs",
        "ts": chrono::Utc::now().to_rfc3339(),
        "by": "diap-rs-sdk-demo"
    });
    let content = serde_json::to_string_pretty(&payload)?;

    println!("开始上传到 IPFS ...");
    let res = client.upload(&content, "ipfs_upload_demo.json").await?;
    println!(
        "上传成功: CID={} size={}B provider={} at={}",
        res.cid, res.size, res.provider, res.uploaded_at
    );

    println!("尝试通过网关读取 ...");
    let fetched = client.get(&res.cid).await?;
    println!("读取成功，内容预览:\n{}", fetched);

    println!("尝试 pin 到远程节点 ...");
    let _ = client.pin(&res.cid).await; // 失败也不影响主要流程
    println!("pin 步骤已完成（若失败请检查节点权限与连通性）");

    Ok(())
}
