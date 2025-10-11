/**
 * IPFS 注册表演示
 * 展示如何发布和查询智能体信息到 IPFS
 */

use diap_rs_sdk::{
    IpfsRegistry, IpfsRegistryConfig, AgentRegistryEntry,
    AgentSearchFilter,
};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("\n🌐 IPFS 注册表演示");
    println!("===================\n");
    
    // 配置 IPFS
    let config = IpfsRegistryConfig {
        api_url: "http://127.0.0.1:5001".to_string(),
        gateway_url: "https://ipfs.io".to_string(),
        pin: true,
    };
    
    let registry = IpfsRegistry::new(config);
    
    println!("📋 选择操作:");
    println!("  1. 发布单个智能体到 IPFS");
    println!("  2. 发布智能体注册表（多个智能体）");
    println!("  3. 查询智能体信息");
    println!("  4. 搜索智能体");
    println!();
    
    // 演示 1: 发布单个智能体
    println!("1️⃣  发布单个智能体到 IPFS\n");
    let agent1 = create_sample_agent("agent1", 3001);
    
    match registry.publish_agent(agent1.clone()).await {
        Ok(cid) => {
            println!("   ✅ 发布成功");
            println!("   📦 CID: {}", cid);
            println!("   🔗 IPFS Gateway: https://ipfs.io/ipfs/{}", cid);
            println!("   🔗 本地网关: http://127.0.0.1:8080/ipfs/{}\n", cid);
            
            // 演示 3: 查询
            println!("3️⃣  从 IPFS 查询智能体信息\n");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            match registry.query_agent(&cid).await {
                Ok(entry) => {
                    println!("   ✅ 查询成功");
                    print_agent_info(&entry);
                }
                Err(e) => {
                    println!("   ⚠️ 查询失败: {}", e);
                    println!("   💡 提示: IPFS 内容可能需要时间传播");
                }
            }
        }
        Err(e) => {
            println!("   ❌ 发布失败: {}", e);
            println!("\n💡 故障排除:");
            println!("   1. 确保 IPFS 节点正在运行:");
            println!("      ipfs daemon");
            println!("   2. 检查 IPFS API 端口 (默认 5001):");
            println!("      curl http://127.0.0.1:5001/api/v0/version");
            println!("   3. 如果没有 IPFS，可以下载:");
            println!("      https://docs.ipfs.tech/install/");
        }
    }
    
    println!("\n");
    
    // 演示 2: 发布注册表索引
    println!("2️⃣  发布智能体注册表（多个智能体）\n");
    let agents = vec![
        create_sample_agent("agent1", 3001),
        create_sample_agent("agent2", 3002),
        create_sample_agent("agent3", 3003),
    ];
    
    match registry.publish_registry_index(agents.clone()).await {
        Ok(index_cid) => {
            println!("   ✅ 注册表发布成功");
            println!("   📦 索引 CID: {}", index_cid);
            println!("   📊 包含 {} 个智能体", agents.len());
            println!("   🔗 IPFS Gateway: https://ipfs.io/ipfs/{}", index_cid);
            println!();
            
            // 演示 4: 搜索
            println!("4️⃣  搜索智能体\n");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // 搜索示例 1: 按能力
            println!("   🔍 搜索具有 NaturalLanguage 能力的智能体:");
            let filter = AgentSearchFilter {
                capabilities: Some(vec!["NaturalLanguage".to_string()]),
                ..Default::default()
            };
            
            match registry.search_agents(&index_cid, filter).await {
                Ok(results) => {
                    println!("   ✅ 找到 {} 个匹配的智能体\n", results.len());
                    for (i, agent) in results.iter().enumerate() {
                        println!("   智能体 {}:", i + 1);
                        print_agent_info(agent);
                        println!();
                    }
                }
                Err(e) => {
                    println!("   ⚠️ 搜索失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ 发布失败: {}", e);
        }
    }
    
    println!("\n💡 使用提示:");
    println!("   - IPFS CID 是内容寻址的哈希，内容不变则 CID 不变");
    println!("   - 可以通过任何 IPFS 网关访问内容");
    println!("   - pin=true 会将内容固定到本地节点");
    println!("   - 可以使用 IPNS 实现可更新的注册表");
    
    Ok(())
}

/// 创建示例智能体
fn create_sample_agent(id: &str, port: u16) -> AgentRegistryEntry {
    AgentRegistryEntry {
        did: format!("did:wba:example.com:{}", id),
        did_web: Some(format!("did:web:example.com:{}", id)),
        name: format!("Demo Agent {}", id),
        endpoint: format!("http://127.0.0.1:{}", port),
        did_document_url: format!("http://127.0.0.1:{}/.well-known/did.json", port),
        ad_url: format!("http://127.0.0.1:{}/agents/{}/ad.json", port, id),
        capabilities: vec![
            "NaturalLanguage".to_string(),
            "DataProcessing".to_string(),
        ],
        interfaces: vec![
            "HTTP".to_string(),
            "WebSocket".to_string(),
        ],
        registered_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

/// 打印智能体信息
fn print_agent_info(entry: &AgentRegistryEntry) {
    println!("      名称: {}", entry.name);
    println!("      DID: {}", entry.did);
    if let Some(ref did_web) = entry.did_web {
        println!("      DID (web): {}", did_web);
    }
    println!("      端点: {}", entry.endpoint);
    println!("      能力: {:?}", entry.capabilities);
    println!("      接口: {:?}", entry.interfaces);
    println!("      注册时间: {}", entry.registered_at);
}

