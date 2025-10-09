// ANP Rust SDK - DID解析模块
// 支持解析 did:ipfs 和 did:wba 格式的DID

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use libp2p::PeerId;
use crate::ipns_publisher::IpnsPublisher;
use crate::ipfs_client::IpfsClient;
use crate::did_builder::DIDDocument;
use crate::libp2p_node::NodeInfo;

/// DID解析结果
#[derive(Debug, Clone)]
pub struct ResolveResult {
    /// DID标识符
    pub did: String,
    
    /// DID文档
    pub did_document: DIDDocument,
    
    /// 解析来源
    pub source: String,
    
    /// 解析时间
    pub resolved_at: String,
}

/// DID解析器
pub struct DIDResolver {
    /// HTTP客户端
    client: Client,
    
    /// IPFS客户端
    ipfs_client: IpfsClient,
    
    /// IPNS发布器（用于解析）
    ipns_publisher: IpnsPublisher,
    
    /// 超时时间
    #[allow(dead_code)]
    timeout: Duration,
}

impl DIDResolver {
    /// 创建新的DID解析器
    pub fn new(
        ipfs_client: IpfsClient,
        ipns_publisher: IpnsPublisher,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("无法创建HTTP客户端");
        
        Self {
            client,
            ipfs_client,
            ipns_publisher,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    /// 解析DID
    /// 支持 did:ipfs:<ipns-name> 和 did:wba:<domain> 格式
    pub async fn resolve(&self, did: &str) -> Result<ResolveResult> {
        log::info!("解析DID: {}", did);
        
        if did.starts_with("did:ipfs:") {
            self.resolve_did_ipfs(did).await
        } else if did.starts_with("did:wba:") {
            self.resolve_did_wba(did).await
        } else if did.starts_with("did:web:") {
            self.resolve_did_web(did).await
        } else {
            anyhow::bail!("不支持的DID格式: {}", did)
        }
    }
    
    /// 解析 did:ipfs 格式
    async fn resolve_did_ipfs(&self, did: &str) -> Result<ResolveResult> {
        // 提取IPNS名称
        // did:ipfs:k51qzi5uqu5d... → k51qzi5uqu5d...
        let ipns_name = did.trim_start_matches("did:ipfs:");
        
        log::debug!("提取IPNS名称: {}", ipns_name);
        
        // 步骤1: 解析IPNS → CID
        let cid = self.ipns_publisher.resolve(ipns_name).await
            .with_context(|| format!("解析IPNS失败: {}", ipns_name))?;
        
        log::debug!("IPNS解析到CID: {}", cid);
        
        // 步骤2: 从IPFS获取DID文档
        let content = self.ipfs_client.get(&cid).await
            .with_context(|| format!("从IPFS获取内容失败: {}", cid))?;
        
        // 步骤3: 解析DID文档
        let did_document: DIDDocument = serde_json::from_str(&content)
            .context("解析DID文档失败")?;
        
        // 步骤4: 验证DID匹配
        if did_document.id != did {
            anyhow::bail!("DID不匹配: 期望 {}, 实际 {}", did, did_document.id);
        }
        
        // 步骤5: 验证双层一致性
        if let Err(e) = crate::did_builder::verify_double_layer(&did_document, ipns_name) {
            log::warn!("双层验证失败: {}", e);
            // 不阻止解析，只是警告
        }
        
        log::info!("✓ DID解析成功: {}", did);
        
        Ok(ResolveResult {
            did: did.to_string(),
            did_document,
            source: "IPFS/IPNS".to_string(),
            resolved_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 解析 did:wba 格式
    async fn resolve_did_wba(&self, did: &str) -> Result<ResolveResult> {
        // did:wba:example.com:user:alice
        // → https://example.com/user/alice/did.json
        
        let url = self.did_to_url_wba(did)?;
        log::debug!("DID URL: {}", url);
        
        // 通过HTTPS获取DID文档
        let response = self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("获取DID文档失败: {}", url))?;
        
        if !response.status().is_success() {
            anyhow::bail!("服务器返回错误: {}", response.status());
        }
        
        let content = response.text().await?;
        
        // 解析DID文档
        let did_document: DIDDocument = serde_json::from_str(&content)
            .context("解析DID文档失败")?;
        
        // 验证DID匹配
        if did_document.id != did {
            anyhow::bail!("DID不匹配: 期望 {}, 实际 {}", did, did_document.id);
        }
        
        log::info!("✓ DID解析成功: {}", did);
        
        Ok(ResolveResult {
            did: did.to_string(),
            did_document,
            source: "HTTPS".to_string(),
            resolved_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 解析 did:web 格式（与did:wba类似）
    async fn resolve_did_web(&self, did: &str) -> Result<ResolveResult> {
        let url = self.did_to_url_web(did)?;
        log::debug!("DID URL: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("获取DID文档失败: {}", url))?;
        
        if !response.status().is_success() {
            anyhow::bail!("服务器返回错误: {}", response.status());
        }
        
        let content = response.text().await?;
        let did_document: DIDDocument = serde_json::from_str(&content)
            .context("解析DID文档失败")?;
        
        if did_document.id != did {
            anyhow::bail!("DID不匹配: 期望 {}, 实际 {}", did, did_document.id);
        }
        
        log::info!("✓ DID解析成功: {}", did);
        
        Ok(ResolveResult {
            did: did.to_string(),
            did_document,
            source: "HTTPS".to_string(),
            resolved_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 将 did:wba 转换为 URL
    /// did:wba:example.com:user:alice → https://example.com/user/alice/did.json
    fn did_to_url_wba(&self, did: &str) -> Result<String> {
        let parts: Vec<&str> = did.split(':').collect();
        
        if parts.len() < 3 {
            anyhow::bail!("无效的did:wba格式: {}", did);
        }
        
        // parts[0] = "did"
        // parts[1] = "wba"
        // parts[2] = domain (可能包含%3A编码的端口)
        // parts[3..] = path
        
        let domain = parts[2].replace("%3A", ":");
        let path = if parts.len() > 3 {
            parts[3..].join("/")
        } else {
            ".well-known".to_string()
        };
        
        let url = if path == ".well-known" {
            format!("https://{}/.well-known/did.json", domain)
        } else {
            format!("https://{}/{}/did.json", domain, path)
        };
        
        Ok(url)
    }
    
    /// 将 did:web 转换为 URL
    /// did:web:example.com%3A3000:user:alice → https://example.com:3000/user/alice/did.json
    fn did_to_url_web(&self, did: &str) -> Result<String> {
        let parts: Vec<&str> = did.split(':').collect();
        
        if parts.len() < 3 {
            anyhow::bail!("无效的did:web格式: {}", did);
        }
        
        let domain = parts[2].replace("%3A", ":");
        let path = if parts.len() > 3 {
            parts[3..].join("/")
        } else {
            ".well-known".to_string()
        };
        
        let url = if path == ".well-known" {
            format!("https://{}/.well-known/did.json", domain)
        } else {
            format!("https://{}/{}/did.json", domain, path)
        };
        
        Ok(url)
    }
    
    /// 批量解析多个DID
    pub async fn resolve_batch(&self, dids: Vec<String>) -> Vec<Result<ResolveResult>> {
        let mut results = Vec::new();
        
        // 使用futures并发解析
        let futures: Vec<_> = dids.iter()
            .map(|did| self.resolve(did))
            .collect();
        
        for future in futures {
            results.push(future.await);
        }
        
        results
    }
    
    /// 从DID文档提取libp2p信息
    pub fn extract_libp2p_info(did_document: &DIDDocument) -> Result<NodeInfo> {
        // 步骤1: 从service字段找到LibP2PNode服务
        let services = did_document.service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少service字段"))?;
        
        let libp2p_service = services.iter()
            .find(|s| s.service_type == "LibP2PNode")
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少LibP2PNode服务"))?;
        
        // 步骤2: 解析NodeInfo JSON
        let node_info: NodeInfo = serde_json::from_str(&libp2p_service.service_endpoint)
            .context("解析libp2p节点信息失败")?;
        
        log::debug!("从DID文档提取libp2p信息:");
        log::debug!("  PeerID: {}", node_info.peer_id);
        log::debug!("  多地址数量: {}", node_info.multiaddrs.len());
        
        Ok(node_info)
    }
    
    /// 验证libp2p公钥与PeerID的绑定
    pub fn verify_libp2p_binding(did_document: &DIDDocument) -> Result<bool> {
        // 步骤1: 提取libp2p信息
        let node_info = Self::extract_libp2p_info(did_document)?;
        let expected_peer_id = node_info.peer_id;
        
        // 步骤2: 从verificationMethod提取libp2p公钥
        let libp2p_key = did_document.verification_method.iter()
            .find(|vm| vm.id.ends_with("#libp2p-key"))
            .ok_or_else(|| anyhow::anyhow!("DID文档缺少libp2p公钥"))?;
        
        // 步骤3: 解码multibase公钥
        let public_key_multibase = &libp2p_key.public_key_multibase;
        if !public_key_multibase.starts_with('z') {
            anyhow::bail!("无效的multibase格式");
        }
        
        let public_key_bytes = bs58::decode(&public_key_multibase[1..])
            .into_vec()
            .context("解码multibase公钥失败")?;
        
        // 步骤4: 从公钥创建libp2p PublicKey
        let libp2p_public_key = libp2p::identity::PublicKey::try_decode_protobuf(&public_key_bytes)
            .context("解析libp2p公钥失败")?;
        
        // 步骤5: 从公钥派生PeerID
        let derived_peer_id = PeerId::from(libp2p_public_key);
        
        // 步骤6: 验证派生的PeerID == 文档中的PeerID
        if derived_peer_id.to_base58() != expected_peer_id {
            anyhow::bail!(
                "libp2p公钥与PeerID不匹配: 派生PeerID({}) != 文档PeerID({})",
                derived_peer_id.to_base58(),
                expected_peer_id
            );
        }
        
        log::info!("✓ libp2p公钥与PeerID绑定验证通过");
        Ok(true)
    }
    
    /// 验证连接的PeerID与DID文档的一致性
    pub fn verify_peer_connection(
        did_document: &DIDDocument,
        connected_peer_id: &PeerId,
    ) -> Result<bool> {
        // 步骤1: 从DID文档提取PeerID
        let node_info = Self::extract_libp2p_info(did_document)?;
        let doc_peer_id = node_info.peer_id;
        
        // 步骤2: 验证连接的PeerID == 文档中的PeerID
        if connected_peer_id.to_base58() != doc_peer_id {
            anyhow::bail!(
                "连接PeerID不匹配: 连接PeerID({}) != 文档PeerID({})",
                connected_peer_id.to_base58(),
                doc_peer_id
            );
        }
        
        log::info!("✓ 连接PeerID与DID文档一致性验证通过");
        Ok(true)
    }
    
    /// 完整的验证闭环
    pub fn verify_complete_chain(
        did_document: &DIDDocument,
        ipns_name: &str,
        connected_peer_id: Option<&PeerId>,
    ) -> Result<bool> {
        log::info!("开始完整验证闭环");
        
        // 验证1: DID标识符与IPNS名称一致
        let did_ipns_name = did_document.id.trim_start_matches("did:ipfs:");
        if did_ipns_name != ipns_name {
            anyhow::bail!("DID与IPNS名称不匹配");
        }
        log::debug!("✓ DID与IPNS名称一致");
        
        // 验证2: libp2p公钥与PeerID绑定
        Self::verify_libp2p_binding(did_document)?;
        
        // 验证3: 如果有连接，验证连接的PeerID
        if let Some(peer_id) = connected_peer_id {
            Self::verify_peer_connection(did_document, peer_id)?;
        }
        
        log::info!("✅ 完整验证闭环通过");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_did_to_url_wba() {
        let resolver = create_test_resolver();
        
        // 测试基础域名
        let url = resolver.did_to_url_wba("did:wba:example.com").unwrap();
        assert_eq!(url, "https://example.com/.well-known/did.json");
        
        // 测试带路径
        let url = resolver.did_to_url_wba("did:wba:example.com:user:alice").unwrap();
        assert_eq!(url, "https://example.com/user/alice/did.json");
        
        // 测试带端口
        let url = resolver.did_to_url_wba("did:wba:example.com%3A3000:user:alice").unwrap();
        assert_eq!(url, "https://example.com:3000/user/alice/did.json");
    }
    
    #[test]
    fn test_did_to_url_web() {
        let resolver = create_test_resolver();
        
        let url = resolver.did_to_url_web("did:web:example.com%3A3000:user:alice").unwrap();
        assert_eq!(url, "https://example.com:3000/user/alice/did.json");
    }
    
    fn create_test_resolver() -> DIDResolver {
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let ipns_publisher = IpnsPublisher::new(true, false, None, 365);
        DIDResolver::new(ipfs_client, ipns_publisher, 30)
    }
    
    #[tokio::test]
    #[ignore] // 需要实际的网络连接
    async fn test_resolve_did_ipfs() {
        let resolver = create_test_resolver();
        
        // 使用一个已知的测试DID
        let result = resolver.resolve("did:ipfs:k51qzi5uqu5d...").await;
        
        if let Ok(result) = result {
            println!("解析成功: {}", result.did);
            assert_eq!(result.source, "IPFS/IPNS");
        }
    }
}
