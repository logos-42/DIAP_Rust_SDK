// ANP Rust SDK - IPNS发布模块
// 支持w3name（优先）和IPFS节点（备用）

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::key_manager::KeyPair;
use base64::{Engine as _, engine::general_purpose};

/// IPNS发布结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpnsPublishResult {
    /// IPNS名称
    pub ipns_name: String,
    
    /// 指向的CID
    pub cid: String,
    
    /// 序列号
    pub sequence: u64,
    
    /// 有效期
    pub validity: String,
    
    /// 发布时间
    pub published_at: String,
    
    /// 使用的方法
    pub method: String,
}

/// IPNS记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpnsRecord {
    /// 指向的CID
    pub value: String,
    
    /// 序列号
    pub sequence: u64,
    
    /// 有效期（RFC3339格式）
    pub validity: String,
}

/// IPNS发布器
#[derive(Clone)]
pub struct IpnsPublisher {
    /// HTTP客户端
    client: Client,
    
    /// 是否使用w3name
    use_w3name: bool,
    
    /// 是否使用IPFS节点
    use_ipfs_node: bool,
    
    /// IPFS节点API地址
    ipfs_api_url: Option<String>,
    
    /// IPNS记录有效期（天）
    validity_days: u64,
}

impl IpnsPublisher {
    /// 创建新的IPNS发布器
    pub fn new(
        use_w3name: bool,
        use_ipfs_node: bool,
        ipfs_api_url: Option<String>,
        validity_days: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("无法创建HTTP客户端");
        
        Self {
            client,
            use_w3name,
            use_ipfs_node,
            ipfs_api_url,
            validity_days,
        }
    }
    
    /// 发布IPNS记录
    /// 优先使用w3name，失败时回退到IPFS节点
    pub async fn publish(
        &self,
        keypair: &KeyPair,
        cid: &str,
        current_sequence: Option<u64>,
    ) -> Result<IpnsPublishResult> {
        let sequence = current_sequence.map(|s| s + 1).unwrap_or(1);
        
        // 尝试w3name
        if self.use_w3name {
            match self.publish_to_w3name(keypair, cid, sequence).await {
                Ok(result) => {
                    log::info!("成功发布到w3name: {}", result.ipns_name);
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("w3name发布失败: {}, 尝试IPFS节点", e);
                }
            }
        }
        
        // 回退到IPFS节点
        if self.use_ipfs_node {
            if let Some(ref api_url) = self.ipfs_api_url {
                match self.publish_to_ipfs_node(keypair, cid, api_url).await {
                    Ok(result) => {
                        log::info!("成功发布到IPFS节点: {}", result.ipns_name);
                        return Ok(result);
                    }
                    Err(e) => {
                        log::error!("IPFS节点发布失败: {}", e);
                        anyhow::bail!("所有IPNS发布方式都失败");
                    }
                }
            }
        }
        
        anyhow::bail!("未配置任何IPNS发布方式")
    }
    
    /// 发布到w3name
    /// 注意：这是一个占位实现，实际需要集成w3name库
    async fn publish_to_w3name(
        &self,
        keypair: &KeyPair,
        cid: &str,
        sequence: u64,
    ) -> Result<IpnsPublishResult> {
        // TODO: 集成w3name Rust库
        // 当前实现：使用w3name HTTP API
        
        let url = format!("https://name.web3.storage/name/{}", keypair.ipns_name);
        
        // 计算有效期
        let validity = chrono::Utc::now() + chrono::Duration::days(self.validity_days as i64);
        
        // 创建IPNS记录
        let record = IpnsRecord {
            value: format!("/ipfs/{}", cid),
            sequence,
            validity: validity.to_rfc3339(),
        };
        
        // 签名记录
        let record_bytes = self.serialize_ipns_record(&record)?;
        let signature = keypair.sign(&record_bytes)?;
        
        // 构建请求体（简化版本）
        let body = serde_json::json!({
            "value": record.value,
            "sequence": record.sequence,
            "validity": record.validity,
            "signature": general_purpose::STANDARD.encode(&signature),
        });
        
        // 发送请求
        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("发送请求到w3name失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("w3name返回错误 {}: {}", status, error_text);
        }
        
        Ok(IpnsPublishResult {
            ipns_name: keypair.ipns_name.clone(),
            cid: cid.to_string(),
            sequence,
            validity: record.validity,
            published_at: chrono::Utc::now().to_rfc3339(),
            method: "w3name".to_string(),
        })
    }
    
    /// 发布到IPFS节点
    async fn publish_to_ipfs_node(
        &self,
        _keypair: &KeyPair,
        cid: &str,
        api_url: &str,
    ) -> Result<IpnsPublishResult> {
        // 使用IPFS HTTP API的/api/v0/name/publish端点
        let url = format!(
            "{}/api/v0/name/publish?arg={}&lifetime={}h",
            api_url,
            cid,
            self.validity_days * 24
        );
        
        // 注意：这需要私钥已经导入到IPFS节点
        // 实际使用中可能需要先导入密钥
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .context("发送请求到IPFS节点失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("IPFS节点返回错误 {}: {}", status, error_text);
        }
        
        // 解析响应
        let response_json: serde_json::Value = response.json().await
            .context("解析IPFS节点响应失败")?;
        
        let ipns_name = response_json["Name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("响应中缺少Name字段"))?
            .to_string();
        
        Ok(IpnsPublishResult {
            ipns_name,
            cid: cid.to_string(),
            sequence: 1, // IPFS节点API不返回序列号
            validity: chrono::Utc::now().to_rfc3339(),
            published_at: chrono::Utc::now().to_rfc3339(),
            method: "IPFS Node".to_string(),
        })
    }
    
    /// 解析IPNS名称
    pub async fn resolve(&self, ipns_name: &str) -> Result<String> {
        // 尝试w3name API
        if self.use_w3name {
            match self.resolve_from_w3name(ipns_name).await {
                Ok(cid) => return Ok(cid),
                Err(e) => {
                    log::warn!("从w3name解析失败: {}", e);
                }
            }
        }
        
        // 尝试IPFS节点
        if let Some(ref api_url) = self.ipfs_api_url {
            match self.resolve_from_ipfs_node(ipns_name, api_url).await {
                Ok(cid) => return Ok(cid),
                Err(e) => {
                    log::warn!("从IPFS节点解析失败: {}", e);
                }
            }
        }
        
        // 尝试公共IPFS网关
        self.resolve_from_gateway(ipns_name).await
    }
    
    /// 从w3name解析
    async fn resolve_from_w3name(&self, ipns_name: &str) -> Result<String> {
        let url = format!("https://name.web3.storage/name/{}", ipns_name);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("查询w3name失败")?;
        
        if !response.status().is_success() {
            anyhow::bail!("w3name返回错误: {}", response.status());
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let value = response_json["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("响应中缺少value字段"))?;
        
        // value格式: /ipfs/QmXxx...
        let cid = value.trim_start_matches("/ipfs/");
        Ok(cid.to_string())
    }
    
    /// 从IPFS节点解析
    async fn resolve_from_ipfs_node(&self, ipns_name: &str, api_url: &str) -> Result<String> {
        let url = format!("{}/api/v0/name/resolve?arg={}", api_url, ipns_name);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .context("查询IPFS节点失败")?;
        
        if !response.status().is_success() {
            anyhow::bail!("IPFS节点返回错误: {}", response.status());
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let path = response_json["Path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("响应中缺少Path字段"))?;
        
        let cid = path.trim_start_matches("/ipfs/");
        Ok(cid.to_string())
    }
    
    /// 从公共网关解析
    async fn resolve_from_gateway(&self, ipns_name: &str) -> Result<String> {
        // 使用IPFS网关的重定向来获取CID
        let url = format!("https://ipfs.io/ipns/{}", ipns_name);
        
        let response = self.client
            .head(&url)
            .send()
            .await
            .context("查询IPFS网关失败")?;
        
        // 从X-Ipfs-Path头获取CID
        if let Some(ipfs_path) = response.headers().get("X-Ipfs-Path") {
            let path = ipfs_path.to_str()?;
            let cid = path.trim_start_matches("/ipfs/");
            return Ok(cid.to_string());
        }
        
        anyhow::bail!("无法从网关解析IPNS")
    }
    
    /// 序列化IPNS记录（用于签名）
    fn serialize_ipns_record(&self, record: &IpnsRecord) -> Result<Vec<u8>> {
        // 简化实现：使用JSON序列化
        // 实际应该使用protobuf格式
        let json = serde_json::to_string(record)?;
        Ok(json.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_manager::KeyPair;
    
    #[test]
    fn test_ipns_publisher_creation() {
        let publisher = IpnsPublisher::new(
            true,
            true,
            Some("http://localhost:5001".to_string()),
            365,
        );
        
        assert!(publisher.use_w3name);
        assert!(publisher.use_ipfs_node);
    }
    
    #[tokio::test]
    #[ignore] // 需要实际的w3name服务
    async fn test_resolve_from_w3name() {
        let publisher = IpnsPublisher::new(true, false, None, 365);
        
        // 使用一个已知的IPNS名称进行测试
        // 注意：这需要一个实际存在的IPNS记录
        let result = publisher.resolve_from_w3name("k51qzi5uqu5d...").await;
        
        if let Ok(cid) = result {
            println!("解析成功，CID: {}", cid);
        }
    }
}
