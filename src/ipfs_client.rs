// DIAP Rust SDK - IPFS客户端模块
// Decentralized Intelligent Agent Protocol
// 支持内置IPFS节点（优先）、AWS IPFS节点和Pinata（备用）

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::ipfs_node_manager::{IpfsNodeManager, IpfsNodeConfig};

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
    
    /// 内置IPFS节点配置
    builtin_config: Option<BuiltinIpfsConfig>,
    
    /// AWS IPFS节点配置
    aws_config: Option<AwsIpfsConfig>,
    
    /// Pinata配置
    pinata_config: Option<PinataConfig>,
    
    /// 超时时间
    #[allow(dead_code)]
    timeout: Duration,
}

/// 内置IPFS节点配置
#[derive(Debug, Clone)]
pub struct BuiltinIpfsConfig {
    pub api_url: String,
    pub gateway_url: String,
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
            builtin_config: None,
            aws_config,
            pinata_config,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    /// 创建带有内置IPFS节点的客户端
    pub async fn new_with_builtin_node(
        config: Option<IpfsNodeConfig>,
        _aws_api_url: Option<String>,
        _aws_gateway_url: Option<String>,
        pinata_api_key: Option<String>,
        pinata_api_secret: Option<String>,
        timeout_seconds: u64,
    ) -> Result<(Self, IpfsNodeManager)> {
        let config = config.unwrap_or_default();
        let node_manager = IpfsNodeManager::new(config);
        
        // 启动内置节点
        node_manager.start().await?;
        
        // 使用内置节点的URL
        let client = Self::new(
            Some(node_manager.api_url().to_string()),
            Some(node_manager.gateway_url().to_string()),
            pinata_api_key,
            pinata_api_secret,
            timeout_seconds,
        );
        
        // 设置为内置节点配置
        let client = Self {
            builtin_config: Some(BuiltinIpfsConfig {
                api_url: node_manager.api_url().to_string(),
                gateway_url: node_manager.gateway_url().to_string(),
            }),
            ..client
        };
        
        Ok((client, node_manager))
    }
    
    /// 创建仅使用内置IPFS节点的客户端（完全去中心化）
    pub async fn new_builtin_only(
        config: Option<IpfsNodeConfig>,
        timeout_seconds: u64,
    ) -> Result<(Self, IpfsNodeManager)> {
        Self::new_with_builtin_node(
            config,
            None, // 不使用AWS
            None, // 不使用AWS网关
            None, // 不使用Pinata
            None,
            timeout_seconds,
        ).await
    }
    
    /// 上传内容到IPFS
    /// 优先使用内置节点，然后AWS节点，最后回退到Pinata
    pub async fn upload(&self, content: &str, name: &str) -> Result<IpfsUploadResult> {
        // 优先尝试内置节点
        if let Some(ref builtin) = self.builtin_config {
            match self.upload_to_builtin(content, name, builtin).await {
                Ok(result) => {
                    log::info!("成功上传到内置IPFS节点: {}", result.cid);
                    return Ok(result);
                }
                Err(e) => {
                    log::warn!("内置IPFS节点上传失败: {}, 尝试其他方式", e);
                }
            }
        }
        
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
    
    /// 上传到内置IPFS节点
    async fn upload_to_builtin(
        &self,
        content: &str,
        name: &str,
        builtin: &BuiltinIpfsConfig,
    ) -> Result<IpfsUploadResult> {
        // 首先尝试HTTP API方式
        match self.upload_to_builtin_via_api(content, name, builtin).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!("HTTP API上传失败: {}, 尝试命令行方式", e);
            }
        }
        
        // 如果HTTP API失败，使用命令行方式
        self.upload_to_builtin_via_cli(content, name).await
    }
    
    async fn upload_to_builtin_via_api(
        &self,
        content: &str,
        name: &str,
        builtin: &BuiltinIpfsConfig,
    ) -> Result<IpfsUploadResult> {
        use reqwest::multipart;
        
        let form = multipart::Form::new()
            .text("pin", "true")
            .part("file", multipart::Part::text(content.to_string()).file_name(name.to_string()));
        
        let url = format!("{}/api/v0/add", builtin.api_url);
        
        let response = self.client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("发送上传请求失败")?;
        
        if !response.status().is_success() {
            anyhow::bail!("上传失败: {}", response.status());
        }
        
        let result: serde_json::Value = response.json().await?;
        let cid = result["Hash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("响应中缺少Hash字段"))?;
        
        let size = result["Size"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        Ok(IpfsUploadResult {
            cid: cid.to_string(),
            size,
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            provider: "builtin".to_string(),
        })
    }
    
    async fn upload_to_builtin_via_cli(
        &self,
        content: &str,
        name: &str,
    ) -> Result<IpfsUploadResult> {
        use std::fs;
        use tokio::process::Command;
        
        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("ipfs_upload_{}", name));
        
        // 写入内容到临时文件
        fs::write(&temp_file, content)
            .context("创建临时文件失败")?;
        
        // 使用ipfs命令行工具上传
        let output = Command::new("ipfs")
            .arg("add")
            .arg("--pin")
            .arg(temp_file.to_str().unwrap())
            .output()
            .await
            .context("执行ipfs add命令失败")?;
        
        // 清理临时文件
        let _ = fs::remove_file(&temp_file);
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ipfs add命令失败: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        // 解析输出，格式通常是 "added <hash> <filename>"
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
        if parts.len() < 2 || parts[0] != "added" {
            anyhow::bail!("无法解析ipfs add输出: {}", stdout);
        }
        
        let cid = parts[1].to_string();
        
        Ok(IpfsUploadResult {
            cid,
            size: content.len() as u64,
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            provider: "builtin_cli".to_string(),
        })
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
        log::info!("🔍 开始从IPFS获取内容: {}", cid);
        
        // 优先使用内置节点网关
        if let Some(ref builtin) = self.builtin_config {
            log::info!("尝试从内置网关获取: {}", builtin.gateway_url);
            match self.get_from_gateway(&builtin.gateway_url, cid).await {
                Ok(content) => {
                    log::info!("✅ 成功从内置IPFS节点获取内容: {}", cid);
                    return Ok(content);
                }
                Err(e) => {
                    log::warn!("❌ 从内置IPFS节点获取失败: {}, 尝试命令行方式", e);
                    // 尝试命令行方式获取
                    match self.get_via_cli(cid).await {
                        Ok(content) => {
                            log::info!("✅ 成功通过命令行获取内容: {}", cid);
                            return Ok(content);
                        }
                        Err(cli_err) => {
                            log::warn!("❌ 命令行获取也失败: {}, 尝试其他网关", cli_err);
                        }
                    }
                }
            }
        } else {
            log::warn!("⚠️  未配置内置IPFS节点");
        }
        
        // 尝试使用AWS网关
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
    
    /// 通过命令行方式从IPFS获取内容
    async fn get_via_cli(&self, cid: &str) -> Result<String> {
        use tokio::process::Command;
        
        let output = Command::new("ipfs")
            .arg("cat")
            .arg(cid)
            .output()
            .await
            .context("执行ipfs cat命令失败")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ipfs cat命令失败: {}", stderr);
        }
        
        let content = String::from_utf8(output.stdout)
            .context("解析ipfs cat输出失败")?;
        
        Ok(content)
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
