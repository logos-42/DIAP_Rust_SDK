// DIAP Rust SDK - IPFS客户端模块 (Helia分支 - 轻量级版本)
// Decentralized Intelligent Agent Protocol
// 边缘服务器专用：仅使用HTTP客户端，无需本地IPFS守护进程

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

/// IPNS发布结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpnsPublishResult {
    /// IPNS名称（PeerID）
    pub name: String,
    
    /// IPNS值（/ipfs/<CID>路径）
    pub value: String,
    
    /// 发布时间
    pub published_at: String,
}

/// IPFS客户端（轻量级版本）
/// 专为边缘服务器设计，只使用HTTP客户端连接到远程IPFS节点
#[derive(Clone)]
pub struct IpfsClient {
    /// HTTP客户端
    client: Client,
    
    /// 远程IPFS API配置
    api_config: Option<RemoteIpfsConfig>,
    
    /// Pinata配置
    pinata_config: Option<PinataConfig>,
    
    /// 公共网关列表
    public_gateways: Vec<String>,
    
    /// 超时时间
    #[allow(dead_code)]
    timeout: Duration,
}

/// 远程IPFS节点配置
#[derive(Debug, Clone)]
pub struct RemoteIpfsConfig {
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
    /// 创建新的IPFS客户端（轻量级版本）
    /// 仅使用HTTP客户端，无需本地守护进程
    pub fn new(
        api_url: Option<String>,
        gateway_url: Option<String>,
        pinata_api_key: Option<String>,
        pinata_api_secret: Option<String>,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .no_proxy() // 避免受系统代理影响导致本地API出现 502/403
            .http1_only() // 与 Kubo 本地 API 更稳定
            .build()
            .expect("无法创建HTTP客户端");
        
        let api_config = if let (Some(api), Some(gateway)) = (api_url, gateway_url) {
            Some(RemoteIpfsConfig {
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
        
        // 默认公共网关列表
        let public_gateways = vec![
            "https://ipfs.io".to_string(),
            "https://dweb.link".to_string(),
            "https://cloudflare-ipfs.com".to_string(),
        ];
        
        Self {
            client,
            api_config,
            pinata_config,
            public_gateways,
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    /// 创建仅使用公共网关的客户端（最轻量级）
    pub fn new_public_only(timeout_seconds: u64) -> Self {
        Self::new(None, None, None, None, timeout_seconds)
    }
    
    /// 创建仅使用远程IPFS节点的客户端
    pub fn new_with_remote_node(
        api_url: String,
        gateway_url: String,
        timeout_seconds: u64,
    ) -> Self {
        Self::new(Some(api_url), Some(gateway_url), None, None, timeout_seconds)
    }
    
    /// 上传内容到IPFS
    /// 如果配置了远程API节点，优先且只使用远程节点（失败则返回具体错误，不再回退）
    pub async fn upload(&self, content: &str, name: &str) -> Result<IpfsUploadResult> {
        if let Some(ref api_config) = self.api_config {
            // 如果配置了远程API，失败时直接返回详细错误
            let result = self.upload_to_remote_api(content, name, api_config).await?;
            return Ok(result);
        }
        // 未配置远程API时，尝试Pinata（如果存在）
        if let Some(ref pinata) = self.pinata_config {
            let result = self.upload_to_pinata(content, name, pinata).await?;
            return Ok(result);
        }
        anyhow::bail!("未配置任何IPFS上传方式：缺少远程API或Pinata凭据")
    }
    
    /// 上传到远程IPFS API节点
    async fn upload_to_remote_api(
        &self,
        content: &str,
        name: &str,
        config: &RemoteIpfsConfig,
    ) -> Result<IpfsUploadResult> {
        use reqwest::multipart;
        
        // 使用bytes形式构造multipart，与 curl -F 行为等价
        let part = multipart::Part::bytes(content.as_bytes().to_vec())
            .file_name(name.to_string())
            .mime_str("application/json").unwrap();
        let form = multipart::Form::new().part("file", part);
        
        // 将 pin=true 放到查询参数，避免作为表单字段被某些代理屏蔽
        let url = format!("{}/api/v0/add?pin=true", config.api_url);
        
        let response = self.client
            .post(&url)
            .header("Expect", "")
            .header("User-Agent", "diap-rs-sdk/0.2")
            .header("Connection", "close")
            .multipart(form)
            .send()
            .await
            .context(format!("发送上传请求失败: {}", url))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("上传失败: {} - {}", status, text);
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
            provider: "remote_api".to_string(),
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
        
        // 优先使用配置的网关
        if let Some(ref api_config) = self.api_config {
            log::info!("尝试从配置网关获取: {}", api_config.gateway_url);
            match self.get_from_gateway(&api_config.gateway_url, cid).await {
                Ok(content) => {
                    log::info!("✅ 成功从配置网关获取内容: {}", cid);
                    return Ok(content);
                }
                Err(e) => {
                    log::warn!("❌ 从配置网关获取失败: {}", e);
                }
            }
        }
        
        // 使用公共IPFS网关
        for gateway in &self.public_gateways {
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
    
    /// Pin内容到远程IPFS节点
    pub async fn pin(&self, cid: &str) -> Result<()> {
        if let Some(ref api_config) = self.api_config {
            let url = format!("{}/api/v0/pin/add?arg={}", api_config.api_url, cid);
            
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
            log::warn!("未配置远程IPFS节点，跳过pin操作");
            Ok(())
        }
    }

    /// 确保命名 key 存在，返回 key 名称（与传入相同）
    pub async fn ensure_key_exists(&self, key_name: &str) -> Result<String> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程IPFS API，无法进行IPNS key 管理"),
        };
        // 列出现有 key
        let url_list = format!("{}/api/v0/key/list", api);
        let resp = self.client.post(&url_list).send().await
            .context("请求 key/list 失败")?;
        if !resp.status().is_success() {
            anyhow::bail!("key/list 失败: {}", resp.status());
        }
        let v: serde_json::Value = resp.json().await?;
        if let Some(arr) = v.get("Keys").and_then(|x| x.as_array()) {
            let exists = arr.iter().any(|k| k.get("Name").and_then(|n| n.as_str()) == Some(key_name));
            if exists { return Ok(key_name.to_string()); }
        }
        // 生成新 key（ed25519）
        let url_gen = format!("{}/api/v0/key/gen?arg={}&type=ed25519", api, urlencoding::encode(key_name));
        let resp_gen = self.client.post(&url_gen).send().await
            .context("请求 key/gen 失败")?;
        if !resp_gen.status().is_success() {
            let status = resp_gen.status();
            let t = resp_gen.text().await.unwrap_or_default();
            anyhow::bail!("key/gen 失败: {} - {}", status, t);
        }
        Ok(key_name.to_string())
    }

    /// 发布 IPNS 记录
    pub async fn publish_ipns(&self, cid: &str, key_name: &str, lifetime: &str, ttl: &str) -> Result<IpnsPublishResult> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程IPFS API，无法进行IPNS发布"),
        };
        let arg_path = format!("/ipfs/{}", cid);
        let url = format!(
            "{}/api/v0/name/publish?arg={}&key={}&allow-offline=true&resolve=false&lifetime={}&ttl={}",
            api,
            urlencoding::encode(&arg_path),
            urlencoding::encode(key_name),
            urlencoding::encode(lifetime),
            urlencoding::encode(ttl)
        );
        let resp = self.client.post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send().await.context("发送 IPNS 发布请求失败")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("IPNS 发布失败: {} - {}", status, t);
        }
        let v: serde_json::Value = resp.json().await?;
        let name = v.get("Name").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        let value = v.get("Value").and_then(|x| x.as_str()).unwrap_or_default().to_string();
        Ok(IpnsPublishResult { name, value, published_at: chrono::Utc::now().to_rfc3339() })
    }

    /// 便捷：上传后发布到 IPNS（需要提前设置 api_url）
    pub async fn publish_after_upload(&self, cid: &str, key_name: &str, lifetime: &str, ttl: &str) -> Result<IpnsPublishResult> {
        let key = self.ensure_key_exists(key_name).await?;
        self.publish_ipns(cid, &key, lifetime, ttl).await
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
        
        assert!(client.api_config.is_some());
        assert!(client.pinata_config.is_none());
    }
    
    #[tokio::test]
    async fn test_ipfs_client_public_only() {
        let client = IpfsClient::new_public_only(30);
        assert!(client.api_config.is_none());
        assert!(!client.public_gateways.is_empty());
    }
    
    // 注意：以下测试需要实际的IPFS节点或Pinata凭证
    // 在CI环境中应该使用mock
}
