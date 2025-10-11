/**
 * ANP IPFS 注册表模块
 * 提供智能体注册到 IPFS 网络的功能
 */

use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::info;
use chrono::Utc;

/// IPFS 注册表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsRegistryConfig {
    /// IPFS HTTP API 地址
    pub api_url: String,
    /// IPFS 网关地址
    pub gateway_url: String,
    /// 是否固定内容（pin）
    pub pin: bool,
}

impl Default for IpfsRegistryConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:5001".to_string(),
            gateway_url: "https://ipfs.io".to_string(),
            pin: true,
        }
    }
}

/// 智能体注册信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistryEntry {
    pub did: String,
    pub did_web: Option<String>,
    pub name: String,
    pub endpoint: String,
    pub did_document_url: String,
    pub ad_url: String,
    pub capabilities: Vec<String>,
    pub interfaces: Vec<String>,
    pub registered_at: String,
    pub updated_at: String,
}

/// IPFS 注册表
pub struct IpfsRegistry {
    config: IpfsRegistryConfig,
    client: reqwest::Client,
}

impl IpfsRegistry {
    /// 创建新的 IPFS 注册表实例
    pub fn new(config: IpfsRegistryConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// 发布智能体信息到 IPFS
    pub async fn publish_agent(&self, entry: AgentRegistryEntry) -> Result<String> {
        info!(" 发布智能体信息到 IPFS: {}", entry.did);
        
        // 序列化智能体信息为 JSON
        let json_data = serde_json::to_string_pretty(&entry)?;
        
        // 上传到 IPFS
        let ipfs_url = format!("{}/api/v0/add", self.config.api_url);
        
        let form = reqwest::multipart::Form::new()
            .text("file", json_data);
        
        let response = self.client
            .post(&ipfs_url)
            .query(&[("pin", self.config.pin.to_string())])
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "IPFS 上传失败: HTTP {}",
                response.status()
            ));
        }
        
        // 解析响应获取 CID
        let ipfs_response: serde_json::Value = response.json().await?;
        let cid = ipfs_response["Hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("IPFS 响应中未找到 Hash"))?
            .to_string();
        
        info!(" 智能体信息已发布到 IPFS: {}", cid);
        info!(" 访问地址: {}/ipfs/{}", self.config.gateway_url, cid);
        
        Ok(cid)
    }

    /// 从 IPFS 查询智能体信息
    pub async fn query_agent(&self, cid: &str) -> Result<AgentRegistryEntry> {
        info!("🔍 从 IPFS 查询智能体信息: {}", cid);
        
        let ipfs_url = format!("{}/ipfs/{}", self.config.gateway_url, cid);
        
        let response = self.client.get(&ipfs_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "IPFS 查询失败: HTTP {}",
                response.status()
            ));
        }
        
        let entry: AgentRegistryEntry = response.json().await?;
        info!(" 成功查询智能体: {}", entry.did);
        
        Ok(entry)
    }

    /// 发布注册表索引（多个智能体的列表）
    pub async fn publish_registry_index(&self, entries: Vec<AgentRegistryEntry>) -> Result<String> {
        info!("发布注册表索引到 IPFS，共 {} 个智能体", entries.len());
        
        let registry_index = RegistryIndex {
            version: "1.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            total_agents: entries.len(),
            agents: entries,
        };
        
        let json_data = serde_json::to_string_pretty(&registry_index)?;
        
        let ipfs_url = format!("{}/api/v0/add", self.config.api_url);
        
        let form = reqwest::multipart::Form::new()
            .text("file", json_data);
        
        let response = self.client
            .post(&ipfs_url)
            .query(&[("pin", self.config.pin.to_string())])
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "IPFS 上传失败: HTTP {}",
                response.status()
            ));
        }
        
        let ipfs_response: serde_json::Value = response.json().await?;
        let cid = ipfs_response["Hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("IPFS 响应中未找到 Hash"))?
            .to_string();
        
        info!("✅ 注册表索引已发布到 IPFS: {}", cid);
        info!("🔗 访问地址: {}/ipfs/{}", self.config.gateway_url, cid);
        
        Ok(cid)
    }

    /// 从 IPFS 查询注册表索引
    pub async fn query_registry_index(&self, cid: &str) -> Result<RegistryIndex> {
        info!("🔍 从 IPFS 查询注册表索引: {}", cid);
        
        let ipfs_url = format!("{}/ipfs/{}", self.config.gateway_url, cid);
        
        let response = self.client.get(&ipfs_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "IPFS 查询失败: HTTP {}",
                response.status()
            ));
        }
        
        let index: RegistryIndex = response.json().await?;
        info!("✅ 成功查询注册表，共 {} 个智能体", index.total_agents);
        
        Ok(index)
    }

    /// 查询智能体（带筛选条件）
    pub async fn search_agents(
        &self,
        index_cid: &str,
        filter: AgentSearchFilter,
    ) -> Result<Vec<AgentRegistryEntry>> {
        let index = self.query_registry_index(index_cid).await?;
        
        let filtered: Vec<AgentRegistryEntry> = index
            .agents
            .into_iter()
            .filter(|agent| {
                // 按 DID 过滤
                if let Some(ref did) = filter.did {
                    if !agent.did.contains(did) && agent.did_web.as_ref().map(|w| !w.contains(did)).unwrap_or(true) {
                        return false;
                    }
                }
                
                // 按能力过滤
                if let Some(ref capabilities) = filter.capabilities {
                    if !capabilities.iter().any(|c| agent.capabilities.contains(c)) {
                        return false;
                    }
                }
                
                // 按接口类型过滤
                if let Some(ref interfaces) = filter.interfaces {
                    if !interfaces.iter().any(|i| agent.interfaces.contains(i)) {
                        return false;
                    }
                }
                
                true
            })
            .collect();
        
        info!("🔍 找到 {} 个匹配的智能体", filtered.len());
        Ok(filtered)
    }
}

/// 注册表索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub version: String,
    pub created_at: String,
    pub total_agents: usize,
    pub agents: Vec<AgentRegistryEntry>,
}

/// 智能体搜索过滤器
#[derive(Debug, Clone, Default)]
pub struct AgentSearchFilter {
    pub did: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub interfaces: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要本地 IPFS 节点才能运行
    async fn test_publish_agent() {
        let config = IpfsRegistryConfig::default();
        let registry = IpfsRegistry::new(config);
        
        let entry = AgentRegistryEntry {
            did: "did:wba:example.com:test".to_string(),
            did_web: Some("did:web:example.com:test".to_string()),
            name: "Test Agent".to_string(),
            endpoint: "http://example.com:3000".to_string(),
            did_document_url: "http://example.com:3000/.well-known/did.json".to_string(),
            ad_url: "http://example.com:3000/agents/test/ad.json".to_string(),
            capabilities: vec!["NaturalLanguage".to_string()],
            interfaces: vec!["HTTP".to_string()],
            registered_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };
        
        let cid = registry.publish_agent(entry).await;
        assert!(cid.is_ok());
        
        println!("Published CID: {}", cid.unwrap());
    }
}

