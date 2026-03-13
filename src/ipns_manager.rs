// IPNS Manager - 独立的 IPNS 管理模块
// 负责 IPNS 记录的发布、解析和 Key 管理

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;

/// IPNS 发布结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpnsPublishResult {
    /// IPNS 名称（PeerID）
    pub name: String,
    /// IPNS 值（/ipfs/<CID> 路径）
    pub value: String,
    /// 发布时间
    pub published_at: String,
}

/// IPNS Key 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// Key 名称
    pub name: String,
    /// Key ID（PeerID）
    pub id: String,
}

/// 远程 IPFS 节点配置（内部使用）
#[derive(Debug, Clone)]
struct RemoteIpfsConfig {
    api_url: String,
    #[allow(dead_code)]
    gateway_url: String,
}

/// IPNS 管理器
/// 
/// 提供 IPNS 记录的发布、解析和 Key 管理功能
/// 
/// # 示例
/// ```rust
/// let ipns = IpnsManager::new_with_api(
///     "http://localhost:5001".to_string(),
///     "http://localhost:8080".to_string(),
/// );
/// 
/// // 发布 IPNS 记录
/// let result = ipns.publish(&cid, "my-key", "8760h", "1h").await?;
/// 
/// // 解析 IPNS 名称
/// let cid = ipns.resolve(&result.name).await?;
/// ```
pub struct IpnsManager {
    /// HTTP 客户端
    client: Client,
    /// 远程 IPFS API 配置
    api_config: Option<RemoteIpfsConfig>,
    /// 超时时间
    timeout: Duration,
    /// Key 缓存（名称 -> PeerID）
    key_cache: Arc<DashMap<String, String>>,
}

impl IpnsManager {
    /// 创建新的 IPNS 管理器
    pub fn new(
        api_url: Option<String>,
        gateway_url: Option<String>,
        timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .no_proxy()
            .http1_only()
            .build()
            .expect("无法创建 HTTP 客户端");

        let api_config = if let (Some(api), Some(gateway)) = (api_url, gateway_url) {
            Some(RemoteIpfsConfig {
                api_url: api,
                gateway_url: gateway,
            })
        } else {
            None
        };

        Self {
            client,
            api_config,
            timeout: Duration::from_secs(timeout_seconds),
            key_cache: Arc::new(DashMap::new()),
        }
    }

    /// 创建带有远程 API 的 IPNS 管理器
    pub fn new_with_api(api_url: String, gateway_url: String) -> Self {
        Self::new(Some(api_url), Some(gateway_url), 30)
    }

    /// 创建仅用于本地的 IPNS 管理器
    pub fn new_local() -> Self {
        Self::new_with_api(
            "http://localhost:5001".to_string(),
            "http://localhost:8080".to_string(),
        )
    }

    /// 检查是否配置了远程 API
    pub fn has_api(&self) -> bool {
        self.api_config.is_some()
    }

    // ==================== IPNS 发布 ====================

    /// 发布 IPNS 记录（普通模式，allow-offline=true）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `key_name`: IPNS key 名称
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// - `ttl`: 缓存时间（如 "1h"）
    pub async fn publish(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
        self.publish_with_mode(cid, key_name, lifetime, ttl, true, false).await
    }

    /// 直接发布 IPNS 记录到 DHT（allow-offline=false）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `key_name`: IPNS key 名称
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
    /// - `ttl`: 缓存时间（如 "1h"）
    /// 
    /// # 说明
    /// 使用 allow-offline=false，要求节点在线并连接到 DHT 网络
    /// 确保记录立即传播到 DHT
    pub async fn publish_direct(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
    ) -> Result<IpnsPublishResult> {
        self.publish_with_mode(cid, key_name, lifetime, ttl, false, true).await
    }

    /// 内部方法：根据模式发布 IPNS
    async fn publish_with_mode(
        &self,
        cid: &str,
        key_name: &str,
        lifetime: &str,
        ttl: &str,
        allow_offline: bool,
        resolve: bool,
    ) -> Result<IpnsPublishResult> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API，无法进行 IPNS 发布"),
        };

        let arg_path = format!("/ipfs/{}", cid);
        let url = format!(
            "{}/api/v0/name/publish?arg={}&key={}&allow-offline={}&resolve={}&lifetime={}&ttl={}",
            api,
            urlencoding::encode(&arg_path),
            urlencoding::encode(key_name),
            if allow_offline { "true" } else { "false" },
            if resolve { "true" } else { "false" },
            urlencoding::encode(lifetime),
            urlencoding::encode(ttl)
        );

        log::info!(
            "📡 发布 IPNS 记录 (allow-offline={}, resolve={})...",
            if allow_offline { "true" } else { "false" },
            if resolve { "true" } else { "false" }
        );
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

            if t.contains("not connected to network") || t.contains("offline") {
                anyhow::bail!(
                    "IPNS 发布失败：节点未连接到 DHT 网络。\n\
                    提示：1) 确保 IPFS 守护进程正在运行\n\
                          2) 检查节点是否可被其他节点访问\n\
                          3) 等待节点连接到足够的对等节点\n\
                    原始错误：{} - {}",
                    status, t
                );
            }

            anyhow::bail!("IPNS 发布失败：{} - {}", status, t);
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

        let result = IpnsPublishResult {
            name,
            value,
            published_at: chrono::Utc::now().to_rfc3339(),
        };

        // 缓存 key 名称
        self.key_cache.insert(key_name.to_string(), result.name.clone());

        Ok(result)
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

        let results = join_all(tasks).await;
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
        let ipns_result = self.publish_direct(cid, key_name, lifetime, ttl).await?;

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

    /// 批量发布到多个 IPNS key（适用于多身份场景）
    /// 
    /// # 参数
    /// - `cid`: 要发布的 IPFS CID
    /// - `keys`: 多个 IPNS key 名称列表
    /// - `lifetime`: 记录的生命周期（如 "8760h"）
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

    // ==================== IPNS 解析 ====================

    /// 解析 IPNS 名称为 CID
    /// 
    /// # 参数
    /// - `ipns_name`: IPNS 名称（PeerID 或 /ipns/PeerID）
    /// 
    /// # 返回
    /// 返回解析后的 CID
    pub async fn resolve(&self, ipns_name: &str) -> Result<String> {
        // 规范化传入名称
        let name = ipns_name.trim();
        let name = if name.starts_with("/ipns/") {
            &name["/ipns/".len()..]
        } else {
            name
        };

        // 使用远程 API
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
                anyhow::bail!("IPNS 解析失败：{} - {}", status, t);
            }

            let v: serde_json::Value = resp.json().await.context("解析 IPNS 解析响应失败")?;
            let path = v
                .get("Path")
                .and_then(|x| x.as_str())
                .ok_or_else(|| anyhow::anyhow!("IPNS 解析响应缺少 Path 字段"))?;

            // 期望格式为 "/ipfs/<CID>"
            let cid = path
                .strip_prefix("/ipfs/")
                .ok_or_else(|| anyhow::anyhow!("IPNS 解析得到的 Path 非 /ipfs/<CID> 格式：{}", path))?;

            return Ok(cid.to_string());
        }

        anyhow::bail!("未配置远程 IPFS API，无法解析 IPNS")
    }

    // ==================== Key 管理 ====================

    /// 确保命名 key 存在，返回 key 名称
    pub async fn ensure_key_exists(&self, key_name: &str) -> Result<String> {
        // 先检查缓存
        if let Some(cached_id) = self.key_cache.get(key_name) {
            return Ok(cached_id.clone());
        }

        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API"),
        };

        // 检查 key 是否存在
        let check_url = format!("{}/api/v0/key/list?l=true", api);
        let resp = self
            .client
            .post(&check_url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("获取 key 列表失败")?;

        if resp.status().is_success() {
            let keys: serde_json::Value = resp.json().await?;
            if let Some(array) = keys.as_array() {
                for key in array {
                    if let Some(name) = key.get("Name").and_then(|x| x.as_str()) {
                        if name == key_name {
                            if let Some(id) = key.get("Id").and_then(|x| x.as_str()) {
                                self.key_cache.insert(key_name.to_string(), id.to_string());
                                return Ok(key_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // key 不存在，创建
        log::info!("🔑 创建 IPNS key: {}", key_name);
        let create_url = format!(
            "{}/api/v0/key/gen?arg={}&type=Ed25519",
            api,
            urlencoding::encode(key_name)
        );

        let resp = self
            .client
            .post(&create_url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("创建 IPNS key 失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("key/gen 失败：{} - {}", status, t);
        }

        let v: serde_json::Value = resp.json().await?;
        if let Some(id) = v.get("Id").and_then(|x| x.as_str()) {
            self.key_cache.insert(key_name.to_string(), id.to_string());
            log::info!("✅ IPNS key 创建成功：{} -> {}", key_name, id);
        }

        Ok(key_name.to_string())
    }

    /// 列出所有 IPNS keys
    pub async fn list_keys(&self) -> Result<Vec<KeyInfo>> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API"),
        };

        let url = format!("{}/api/v0/key/list?l=true", api);
        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("获取 key 列表失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("key/list 失败：{} - {}", status, t);
        }

        let keys: serde_json::Value = resp.json().await?;
        let mut result = Vec::new();

        if let Some(array) = keys.as_array() {
            for key in array {
                if let (Some(name), Some(id)) = (
                    key.get("Name").and_then(|x| x.as_str()),
                    key.get("Id").and_then(|x| x.as_str()),
                ) {
                    result.push(KeyInfo {
                        name: name.to_string(),
                        id: id.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    /// 删除 IPNS key
    pub async fn remove_key(&self, key_name: &str) -> Result<()> {
        let api = match &self.api_config {
            Some(c) => &c.api_url,
            None => anyhow::bail!("未配置远程 IPFS API"),
        };

        let url = format!(
            "{}/api/v0/key/rm?arg={}",
            api,
            urlencoding::encode(key_name)
        );

        let resp = self
            .client
            .post(&url)
            .header("User-Agent", "diap-rs-sdk/0.2")
            .send()
            .await
            .context("删除 IPNS key 失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("key/rm 失败：{} - {}", status, t);
        }

        // 从缓存中移除
        self.key_cache.remove(key_name);

        Ok(())
    }

    // ==================== 高级功能 ====================

    /// 预传播 CID 到多个节点（加速首次访问）
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

    /// 广播 IPNS 记录到 DHT（加速传播）
    pub async fn broadcast_to_dht(&self, ipns_name: &str) -> Result<()> {
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

    /// 获取缓存的 key 信息
    pub fn get_cached_key(&self, key_name: &str) -> Option<String> {
        self.key_cache.get(key_name).map(|v| v.clone())
    }

    /// 清除 key 缓存
    pub fn clear_key_cache(&self) {
        self.key_cache.clear();
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_keys: self.key_cache.len(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_keys: usize,
}

/// 便捷方法：上传后发布到 IPNS
pub async fn publish_after_upload(
    ipns: &IpnsManager,
    cid: &str,
    key_name: &str,
    lifetime: &str,
    ttl: &str,
) -> Result<IpnsPublishResult> {
    let key = ipns.ensure_key_exists(key_name).await?;
    ipns.publish(cid, &key, lifetime, ttl).await
}

/// 便捷方法：上传后直接发布到 IPNS DHT
pub async fn publish_after_upload_direct(
    ipns: &IpnsManager,
    cid: &str,
    key_name: &str,
    lifetime: &str,
    ttl: &str,
) -> Result<IpnsPublishResult> {
    let key = ipns.ensure_key_exists(key_name).await?;
    ipns.publish_direct(cid, &key, lifetime, ttl).await
}
