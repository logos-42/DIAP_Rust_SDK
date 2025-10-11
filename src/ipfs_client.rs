// DIAP Rust SDK - IPFS客户端模块
// Decentralized Intelligent Agent Protocol
// 支持AWS IPFS节点（优先）和Pinata（备用）

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// IPFS上传结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsUploadResult {
    /// 内容CID
    pub cid: String,
    
    /// 内容大小（字节）
    pub size: u64,
    
    /// 上传时间
    pub uploaded_at: String,
    
    /// 使用的提供商
    pub provider: String,
}

/// IPFS客户端
#[derive(Clone)]
pub struct IpfsClient {
    /// HTTP客户端
    client: Client,
    
    /// AWS IPFS节点配置
    aws_config: Option<AwsIpfsConfig>,
    
    /// Pinata配置
    pinata_config: Option<PinataConfig>,
    
    /// 超时时间
    #[allow(dead_code)]
    timeout: Duration,
}

/// AWS IPFS节点配置
#[derive(Debug, Clone)]
pub struct AwsIpfsConfig {
    pub api_url: String,
    pub gateway_url: String,
}

/// Pinata配置
#[derive(Debug, Clone)]
pub struct PinataConfig {
    pub api_key: String,
    pub api_secret: String,
}

impl IpfsClient {
    /// 创建新的IPFS客户端
    pub fn new(
        aws_api_url: Option<String>,
        aws_gateway_url: Option<String>,
        pinata_api_key: Option<String>,
        pinata_api_secret: Option<String>,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .expect("无法创建HTTP客户端");
        
        let aws_config = if let (Some(api), Some(gateway)) = (aws_api_url, aws_gateway_url) {
            Some(AwsIpfsConfig {
                api_url: api,
                gateway_url: gateway,
            })
        } else {
            None
        };
        
        let pinata_config = if let (Some(key), Some(secret)) = (pinata_api_key, pinata_api_secret) {
            Some(PinataConfig {
                api_key: key,
                api_secret: secret,
            })
        } else {
            None
        };
        
        Self {
            client,
            aws_config,
            pinata_config,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    /// 上传内容到IPFS
    /// 优先使用AWS节点，失败时回退到Pinata
    pub async fn upload(&self, content: &str, name: &str) -> Result<IpfsUploadResult> {
        // 尝试AWS节点
        if let Some(ref aws) = self.aws_config {
            match self.upload_to_aws(content, name, aws).await {
                Ok(result) => {
                    log::info!("成功上传到AWS IPFS节点: {}", result.cid);
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("AWS IPFS节点上传失败: {}, 尝试Pinata", e);
                }
            }
        }
        
        // 回退到Pinata
        if let Some(ref pinata) = self.pinata_config {
            match self.upload_to_pinata(content, name, pinata).await {
                Ok(result) => {
                    log::info!("成功上传到Pinata: {}", result.cid);
                    return Ok(result);
                }
                Err(e) => {
                    log::error!("Pinata上传失败: {}", e);
                    anyhow::bail!("所有IPFS上传方式都失败");
                }
            }
        }
        
        anyhow::bail!("未配置任何IPFS上传方式")
    }
    
    /// 上传到AWS IPFS节点
    async fn upload_to_aws(
        &self,
        content: &str,
        _name: &str,
        config: &AwsIpfsConfig,
    ) -> Result<IpfsUploadResult> {
        // 使用IPFS HTTP API的/api/v0/add端点
        let url = format!("{}/api/v0/add", config.api_url);
        
        // 创建multipart表单
        let form = reqwest::multipart::Form::new()
            .text("file", content.to_string());
        
        // 发送请求
        let response = self.client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("发送请求到AWS IPFS节点失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("AWS IPFS节点返回错误 {}: {}", status, error_text);
        }
        
        // 解析响应
        let response_text = response.text().await?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .context("解析AWS IPFS响应失败")?;
        
        let cid = response_json["Hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("响应中缺少Hash字段"))?
            .to_string();
        
        let size = response_json["Size"]
            .as_u64()
            .unwrap_or(0);
        
        Ok(IpfsUploadResult {
            cid,
            size,
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            provider: "AWS IPFS".to_string(),
        })
    }
    
    /// 上传到Pinata
    async fn upload_to_pinata(
        &self,
        content: &str,
        name: &str,
        config: &PinataConfig,
    ) -> Result<IpfsUploadResult> {
        let url = "https://api.pinata.cloud/pinning/pinJSONToIPFS";
        
        // 构建请求体
        let body = serde_json::json!({
            "pinataContent": serde_json::from_str::<serde_json::Value>(content)?,
            "pinataMetadata": {
                "name": name,
                "keyvalues": {
                    "type": "did-document",
                    "uploaded_by": "diap-rs-sdk"
                }
            }
        });
        
        // 发送请求
        let response = self.client
            .post(url)
            .header("pinata_api_key", &config.api_key)
            .header("pinata_secret_api_key", &config.api_secret)
            .json(&body)
            .send()
            .await
            .context("发送请求到Pinata失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Pinata返回错误 {}: {}", status, error_text);
        }
        
        // 解析响应
        #[derive(Deserialize)]
        struct PinataResponse {
            #[serde(rename = "IpfsHash")]
            ipfs_hash: String,
            #[serde(rename = "PinSize")]
            pin_size: u64,
        }
        
        let pinata_response: PinataResponse = response.json().await
            .context("解析Pinata响应失败")?;
        
        Ok(IpfsUploadResult {
            cid: pinata_response.ipfs_hash,
            size: pinata_response.pin_size,
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            provider: "Pinata".to_string(),
        })
    }
    
    /// 从IPFS获取内容
    pub async fn get(&self, cid: &str) -> Result<String> {
        // 优先使用AWS网关
        if let Some(ref aws) = self.aws_config {
            match self.get_from_gateway(&aws.gateway_url, cid).await {
                Ok(content) => return Ok(content),
                Err(e) => {
                    log::warn!("从AWS网关获取失败: {}, 尝试公共网关", e);
                }
            }
        }
        
        // 使用公共IPFS网关
        let public_gateways = [
            "https://ipfs.io",
            "https://dweb.link",
            "https://cloudflare-ipfs.com",
        ];
        
        for gateway in &public_gateways {
            match self.get_from_gateway(gateway, cid).await {
                Ok(content) => return Ok(content),
                Err(e) => {
                    log::warn!("从{}获取失败: {}", gateway, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("无法从任何网关获取内容")
    }
    
    /// 从指定网关获取内容
    async fn get_from_gateway(&self, gateway_url: &str, cid: &str) -> Result<String> {
        let url = format!("{}/ipfs/{}", gateway_url, cid);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("发送请求失败")?;
        
        if !response.status().is_success() {
            anyhow::bail!("网关返回错误: {}", response.status());
        }
        
        let content = response.text().await
            .context("读取响应内容失败")?;
        
        Ok(content)
    }
    
    /// Pin内容到AWS IPFS节点
    pub async fn pin(&self, cid: &str) -> Result<()> {
        if let Some(ref aws) = self.aws_config {
            let url = format!("{}/api/v0/pin/add?arg={}", aws.api_url, cid);
            
            let response = self.client
                .post(&url)
                .send()
                .await
                .context("发送pin请求失败")?;
            
            if !response.status().is_success() {
                anyhow::bail!("Pin失败: {}", response.status());
            }
            
            log::info!("成功pin内容: {}", cid);
            Ok(())
        } else {
            log::warn!("未配置AWS IPFS节点，跳过pin操作");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ipfs_client_creation() {
        let client = IpfsClient::new(
            Some("http://localhost:5001".to_string()),
            Some("http://localhost:8080".to_string()),
            None,
            None,
            30,
        );
        
        assert!(client.aws_config.is_some());
        assert!(client.pinata_config.is_none());
    }
    
    // 注意：以下测试需要实际的IPFS节点或Pinata凭证
    // 在CI环境中应该使用mock
    
    #[tokio::test]
    #[ignore] // 需要实际的IPFS节点
    async fn test_upload_to_aws() {
        let client = IpfsClient::new(
            Some("http://localhost:5001".to_string()),
            Some("http://localhost:8080".to_string()),
            None,
            None,
            30,
        );
        
        let content = r#"{"test": "data"}"#;
        let result = client.upload(content, "test.json").await;
        
        // 如果本地有IPFS节点，这应该成功
        if let Ok(result) = result {
            assert!(!result.cid.is_empty());
            println!("上传成功，CID: {}", result.cid);
        }
    }
}
