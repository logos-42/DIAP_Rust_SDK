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
        Self::new(
            Some(api_url),
            Some(gateway_url),
            None,
            None,
            timeout_seconds,
        )
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
            .mime_str("application/json")
            .unwrap();
        let form = multipart::Form::new().part("file", part);

        // 将 pin=true 放到查询参数，避免作为表单字段被某些代理屏蔽
        let url = format!("{}/api/v0/add?pin=true", config.api_url);

        let response = self
            .client
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
        let response = self
            .client
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

        let pinata_response: PinataResponse =
            response.json().await.context("解析Pinata响应失败")?;

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
            if let Ok(content) = self.get_from_gateway(&api_config.gateway_url, cid).await {
                log::info!("✅ 成功从配置网关获取内容: {}", cid);
                return Ok(content);
            } else {
                log::warn!("❌ 从配置网关获取失败，尝试使用 API fallback (/api/v0/cat)");
                // 使用 API fallback
                let url = format!("{}/api/v0/cat?arg={}", api_config.api_url, cid);
                let resp = self
                    .client
                    .post(&url)
                    .header("User-Agent", "diap-rs-sdk/0.2")
                    .send()
                    .await
                    .context("发送 API fallback 请求失败")?;
                if resp.status().is_success() {
                    let content = resp.text().await.context("读取 API fallback 响应失败")?;
                    log::info!("✅ 通过 API fallback 获取内容成功: {}", cid);
                    return Ok(content);
                } else {
                    let status = resp.status();
                    let t = resp.text().await.unwrap_or_default();
                    log::warn!("API fallback 失败: {} - {}", status, t);
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

        let response = self.client.get(&url).send().await.context("发送请求失败")?;

        if !response.status().is_success() {
            anyhow::bail!("网关返回错误: {}", response.status());
        }

        let content = response.text().await.context("读取响应内容失败")?;

        Ok(content)
    }

    /// Pin内容到远程IPFS节点
    pub async fn pin(&self, cid: &str) -> Result<()> {
        if let Some(ref api_config) = self.api_config {
            let url = format!("{}/api/v0/pin/add?arg={}", api_config.api_url, cid);

            let response = self
                .client
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
        log::info!("🔑 检查 IPNS key '{}' 是否存在...", key_name);
        let url_list = format!("{}/api/v0/key/list", api);
        log::debug!("   请求 URL: {}", url_list);
        let resp = self
            .client
            .post(&url_list)
            .timeout(self.timeout)
            .send()
            .await
            .context("请求 key/list 失败")?;
        if !resp.status().is_success() {
            anyhow::bail!("key/list 失败: {}", resp.status());
        }
        let v: serde_json::Value = resp.json().await?;
        if let Some(arr) = v.get("Keys").and_then(|x| x.as_array()) {
            let exists = arr
                .iter()
                .any(|k| k.get("Name").and_then(|n| n.as_str()) == Some(key_name));
            if exists {
                return Ok(key_name.to_string());
            }
        }
        // 生成新 key（ed25519）
        log::info!("   创建新的 IPNS key '{}'...", key_name);
        let url_gen = format!(
            "{}/api/v0/key/gen?arg={}&type=ed25519",
            api,
            urlencoding::encode(key_name)
        );
        log::debug!("   请求 URL: {}", url_gen);
        let resp_gen = self
            .client
            .post(&url_gen)
            .timeout(self.timeout)
            .send()
            .await
            .context("请求 key/gen 失败")?;
        if !resp_gen.status().is_success() {
            let status = resp_gen.status();
            let t = resp_gen.text().await.unwrap_or_default();
            anyhow::bail!("key/gen 失败: {} - {}", status, t);
        }
        Ok(key_name.to_string())
    }

    /// 发布 IPNS 记录
    pub async fn publish_ipns(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
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
        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("发送 IPNS 发布请求失败")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("IPNS 发布失败: {} - {}", status, t);
        }
        let v: serde_json::Value = resp.json().await?;
        let name = v
            .get("Name")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let value = v
            .get("Value")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        Ok(IpnsPublishResult {
            name,
            value,
            published_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 直接发布 IPNS 记录到 DHT（要求节点在线）
    /// 使用 allow-offline=false，确保记录立即传播到 DHT 网络
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `key_name`: IPNS key 名称
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// - `ttl`: 缓存时间（如 "1h"）
    /// 
    /// # 返回
    /// 如果节点未连接到 DHT 网络，将返回错误
    pub async fn publish_ipns_direct(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程IPFS API，无法进行IPNS发布"),
        };
        let arg_path = format!("/ipfs/{}", cid);
        
        // 关键改动：allow-offline=false，要求节点在线并连接到DHT
        let url = format!(
            "{}/api/v0/name/publish?arg={}&key={}&allow-offline=false&resolve=true&lifetime={}&ttl={}",
            api,
            urlencoding::encode(&arg_path),
            urlencoding::encode(key_name),
            urlencoding::encode(lifetime),
            urlencoding::encode(ttl)
        );
        
        log::info!("📡 发布IPNS记录到DHT（allow-offline=false）...");
        log::info!("   要求: 节点必须在线并连接到DHT网络");
        log::debug!("   请求 URL: {}", url);
        
        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .timeout(self.timeout)
            .send()
            .await
            .context("发送 IPNS 发布请求失败")?;
            
        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            
            // 如果失败，提供更详细的错误信息
            if t.contains("not connected to network") || t.contains("offline") {
                anyhow::bail!(
                    "IPNS 发布失败: 节点未连接到DHT网络。\n\
                    提示: 1) 确保IPFS守护进程正在运行\n\
                          2) 检查节点是否可被其他节点访问\n\
                          3) 等待节点连接到足够的对等节点\n\
                    原始错误: {} - {}",
                    status, t
                );
            }
            
            anyhow::bail!("IPNS 发布失败: {} - {}", status, t);
        }
        
        let v: serde_json::Value = resp.json().await?;
        let name = v
            .get("Name")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let value = v
            .get("Value")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
            
        log::info!("✅ IPNS记录已发布到DHT: /ipns/{}", name);
        log::info!("   记录将在DHT网络中传播，全球可访问");
        
        Ok(IpnsPublishResult {
            name,
            value,
            published_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 便捷：上传后发布到 IPNS（需要提前设置 api_url）
    pub async fn publish_after_upload(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
        let key = self.ensure_key_exists(key_name).await?;
        self.publish_ipns(cid, &key, lifetime, ttl).await
    }

    /// 便捷：上传后直接发布到 IPNS DHT（需要提前设置 api_url）
    /// 使用 allow-offline=false，确保记录立即传播到 DHT
    pub async fn publish_after_upload_direct(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
        let key = self.ensure_key_exists(key_name).await?;
        self.publish_ipns_direct(cid, &key, lifetime, ttl).await
    }

    /// 并行发布到多个 IPFS 节点（加速传播）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `key_name`: IPNS key 名称
    /// - `nodes`: 多个节点的 API URL 列表
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// - `ttl`: 缓存时间（如 "1h"）
    /// 
    /// # 返回
    /// 返回所有节点的发布结果（成功的结果）
    /// 
    /// # 示例
    /// ```rust
    /// let nodes = vec![
    ///     "http://node1:5001",
    ///     "http://node2:5001",
    ///     "http://node3:5001",
    /// ];
    /// client.publish_to_multiple_nodes(&cid, "my-key", &nodes, "8760h", "1h").await?;
    /// ```
    pub async fn publish_to_multiple_nodes(
        &self,
        cid: &str,
        key_name: &str,
        nodes: Vec<&str>,
        lifetime: &str,
        ttl: &str,
    ) -> Result<Vec<IpnsPublishResult>> {
        use futures::future::join_all;

        log::info!("🚀 并行发布到 {} 个节点...", nodes.len());

        // 为每个节点创建发布任务
        let tasks = nodes.iter().map(|&node_url| {
            let client = &self.client;
            let timeout = self.timeout;
            async move {
                let arg_path = format!("/ipfs/{}", cid);
                let url = format!(
                    "{}/api/v0/name/publish?arg={}&key={}&allow-offline=false&resolve=true&lifetime={}&ttl={}",
                    node_url,
                    urlencoding::encode(&arg_path),
                    urlencoding::encode(key_name),
                    urlencoding::encode(lifetime),
                    urlencoding::encode(ttl)
                );

                let resp = client
                    .post(&url)
                    .header("User-Agent", "diap-rs-sdk/0.2")
                    .timeout(timeout)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let t = resp.text().await.unwrap_or_default();
                    anyhow::bail!("节点 {} IPNS 发布失败：{} - {}", node_url, status, t);
                }

                let v: serde_json::Value = resp.json().await?;
                let name = v
                    .get("Name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let value = v
                    .get("Value")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(IpnsPublishResult {
                    name,
                    value,
                    published_at: chrono::Utc::now().to_rfc3339(),
                })
            }
        });

        // 并行执行所有任务
        let results = join_all(tasks).await;

        // 收集成功的结果
        let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_ok());

        if !failures.is_empty() {
            log::warn!("⚠️ {}/{} 节点发布失败", failures.len(), nodes.len());
            for (i, err) in failures.iter().enumerate() {
                log::warn!("   节点 {}: {}", i + 1, err.as_ref().unwrap_err());
            }
        }

        log::info!("✅ {}/{} 节点发布成功", successes.len(), nodes.len());

        Ok(successes.into_iter().filter_map(|r| r.ok()).collect())
    }

    /// 发布 IPNS 并同时 Pin 到多个节点（确保内容可用性）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `key_name`: IPNS key 名称
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// - `ttl`: 缓存时间（如 "1h"）
    /// - `pin_nodes`: 要 Pin 的节点 API URL 列表
    /// 
    /// # 返回
    /// 返回 IPNS 发布结果
    pub async fn publish_and_pin(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
        pin_nodes: Vec<&str>,
    ) -> Result<IpnsPublishResult> {
        log::info!("📌 发布 IPNS 并 Pin 到 {} 个节点...", pin_nodes.len());

        // 首先发布 IPNS
        let ipns_result = self.publish_after_upload_direct(cid, key_name, lifetime, ttl).await?;

        // 并行 Pin 到所有节点
        if !pin_nodes.is_empty() {
            use futures::future::join_all;

            let pin_tasks = pin_nodes.iter().map(|&node_url| {
                let client = &self.client;
                async move {
                    let url = format!("{}/api/v0/pin/add?arg={}", node_url, cid);
                    let resp = client
                        .post(&url)
                        .header("User-Agent", "diap-rs-sdk/0.2")
                        .send()
                        .await?;

                    if resp.status().is_success() {
                        log::info!("✅ 已 Pin 到节点：{}", node_url);
                        Ok::<_, anyhow::Error>(())
                    } else {
                        let status = resp.status();
                        let t = resp.text().await.unwrap_or_default();
                        log::warn!("⚠️ 节点 {} Pin 失败：{} - {}", node_url, status, t);
                        Ok::<_, anyhow::Error>(()) // 不阻塞主流程
                    }
                }
            });

            join_all(pin_tasks).await;
        }

        Ok(ipns_result)
    }

    /// 预传播 CID 到多个节点（加速首次访问）
    /// 
    /// # 参数
    /// - `cid`: 要传播的 IPFS CID
    /// - `bootstrap_nodes`: 引导节点 API URL 列表
    /// 
    /// # 说明
    /// 通过 Pin 操作让节点主动获取并缓存 CID 内容
    pub async fn prepropagate_cid(
        &self,
        cid: &str,
        bootstrap_nodes: Vec<&str>,
    ) -> Result<()> {
        log::info!("📢 预传播 CID {} 到 {} 个节点...", cid, bootstrap_nodes.len());

        use futures::future::join_all;

        let tasks = bootstrap_nodes.iter().map(|&node_url| {
            let client = &self.client;
            async move {
                // 使用 pin remote add 或 pin add 来触发内容获取
                let url = format!("{}/api/v0/pin/add?arg={}&progress=false", node_url, cid);
                let resp = client
                    .post(&url)
                    .header("User-Agent", "diap-rs-sdk/0.2")
                    .send()
                    .await?;

                if resp.status().is_success() {
                    log::debug!("✅ 节点 {} 已缓存 CID {}", node_url, cid);
                    Ok::<_, anyhow::Error>(())
                } else {
                    let status = resp.status();
                    let t = resp.text().await.unwrap_or_default();
                    log::warn!("⚠️ 节点 {} 缓存失败：{} - {}", node_url, status, t);
                    Ok::<_, anyhow::Error>(())
                }
            }
        });

        join_all(tasks).await;
        log::info!("✅ CID 预传播完成");
        Ok(())
    }

    /// 批量发布到多个 IPNS key（适用于多身份场景）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `keys`: 多个 IPNS key 名称列表
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// 
    /// # 返回
    /// 返回所有 key 的发布结果
    pub async fn batch_publish_ipns(
        &self,
        cid: &str,
        keys: Vec<&str>,
        lifetime: &str,
    ) -> Result<Vec<IpnsPublishResult>> {
        use futures::future::join_all;

        log::info!("🔑 批量发布到 {} 个 IPNS key...", keys.len());

        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API"),
        };

        let tasks = keys.iter().map(|&key_name| {
            let client = &self.client;
            let timeout = self.timeout;
            let api_url = api.clone();
            async move {
                let arg_path = format!("/ipfs/{}", cid);
                let url = format!(
                    "{}/api/v0/name/publish?arg={}&key={}&allow-offline=false&resolve=true&lifetime={}&ttl=1h",
                    api_url,
                    urlencoding::encode(&arg_path),
                    urlencoding::encode(key_name),
                    urlencoding::encode(lifetime)
                );

                let resp = client
                    .post(&url)
                    .header("User-Agent", "diap-rs-sdk/0.2")
                    .timeout(timeout)
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let t = resp.text().await.unwrap_or_default();
                    anyhow::bail!("Key {} IPNS 发布失败：{} - {}", key_name, status, t);
                }

                let v: serde_json::Value = resp.json().await?;
                let name = v
                    .get("Name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                let value = v
                    .get("Value")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(IpnsPublishResult {
                    name,
                    value,
                    published_at: chrono::Utc::now().to_rfc3339(),
                })
            }
        });

        let results = join_all(tasks).await;
        let (successes, failures): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_ok());

        if !failures.is_empty() {
            log::warn!("⚠️ {}/{} key 发布失败", failures.len(), keys.len());
        }

        log::info!("✅ {}/{} key 发布成功", successes.len(), keys.len());

        Ok(successes.into_iter().filter_map(|r| r.ok()).collect())
    }

    /// 广播 IPNS 记录到 DHT（加速传播）
    /// 
    /// # 参数
    /// - `ipns_name`: 要广播的 IPNS 名称（PeerID）
    /// 
    /// # 说明
    /// 通过订阅自己的 IPNS 记录来触发 DHT 广播
    pub async fn broadcast_to_dht(
        &self,
        ipns_name: &str,
    ) -> Result<()> {
        log::info!("📡 广播 IPNS 记录 {} 到 DHT...", ipns_name);

        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API"),
        };

        // 使用 name/pubsub/pub 触发广播
        let url = format!(
            "{}/api/v0/name/pubsub/pub?arg={}",
            api,
            urlencoding::encode(&format!("/ipns/{}", ipns_name))
        );

        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("发送 DHT 广播请求失败")?;

        if resp.status().is_success() {
            log::info!("✅ IPNS 记录已广播到 DHT");
            Ok(())
        } else {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            log::warn!("⚠️ DHT 广播失败：{} - {}", status, t);
            Ok(()) // 不阻塞主流程
        }
    }

    /// 解析 IPNS 名称为 CID
    /// 支持两种方式：
    /// 1) 优先通过远程 IPFS API (/api/v0/name/resolve)
    /// 2) 如果没有配置 API，尝试通过网关的重定向解析（HEAD 请求，不跟随重定向）
    pub async fn resolve_ipns(&self, ipns_name: &str) -> Result<String> {
        // 规范化传入名称，支持 "peerId" 或 "/ipns/peerId"
        let name = ipns_name.trim();
        let name = if name.starts_with("/ipns/") {
            &name["/ipns/".len()..]
        } else {
            name
        };

        // 优先使用远程 API
        if let Some(ref api_config) = self.api_config {
            let url = format!(
                "{}/api/v0/name/resolve?arg={}&recursive=true&nocache=true",
                api_config.api_url,
                urlencoding::encode(&format!("/ipns/{}", name))
            );

            let resp = self
                .client
                .post(&url)
                .header("User-Agent", "diap-rs-sdk/0.2")
                .send()
                .await
                .context("发送 IPNS 解析请求失败")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let t = resp.text().await.unwrap_or_default();
                anyhow::bail!("IPNS 解析失败: {} - {}", status, t);
            }

            let v: serde_json::Value = resp.json().await.context("解析 IPNS 解析响应失败")?;
            let path = v
                .get("Path")
                .and_then(|x| x.as_str())
                .ok_or_else(|| anyhow::anyhow!("IPNS 解析响应缺少 Path 字段"))?;

            // 期望格式为 "/ipfs/<CID>"
            let cid = path
                .strip_prefix("/ipfs/")
                .ok_or_else(|| anyhow::anyhow!("IPNS 解析得到的 Path 非 /ipfs/<CID> 格式: {}", path))?;

            return Ok(cid.to_string());
        }

        // 未配置 API：尝试通过公共网关使用不跟随重定向的 HEAD 请求，读取 Location 头部获取 CID
        // 注意：并非所有网关都支持对 /ipns/<name> 返回重定向，此步骤为尽力而为
        for gateway in &self.public_gateways {
            // 构造一个不跟随重定向的临时客户端
            let tmp_client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(self.timeout)
                .build()
                .context("创建临时HTTP客户端失败")?;

            let url = format!("{}/ipns/{}", gateway, name);
            let resp = tmp_client.head(&url).send().await;
            match resp {
                Ok(r) => {
                    // 3xx 时应包含 Location 指向 /ipfs/<CID>
                    if r.status().is_redirection() {
                        if let Some(loc) = r.headers().get(reqwest::header::LOCATION) {
                            if let Ok(loc_str) = loc.to_str() {
                                if let Some(cid) = loc_str.strip_prefix("/ipfs/") {
                                    return Ok(cid.to_string());
                                }
                                // 某些网关返回绝对URL，尝试查找 "/ipfs/"
                                if let Some(pos) = loc_str.find("/ipfs/") {
                                    let cid_part = &loc_str[pos + "/ipfs/".len()..];
                                    // 提取到下一个分隔符结束
                                    let cid = cid_part
                                        .split(|c| c == '/' || c == '?' || c == '#')
                                        .next()
                                        .unwrap_or(cid_part);
                                    if !cid.is_empty() {
                                        return Ok(cid.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("通过网关解析 IPNS 失败 ({}): {}", gateway, e);
                    continue;
                }
            }
        }

        anyhow::bail!("未配置远程 IPFS API，且无法通过任何网关解析 IPNS 名称")
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
