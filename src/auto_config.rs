/**
 * DIAP协议自动配置模块 - Rust版本
 * Decentralized Intelligent Agent Protocol
 * 提供端口自动分配、DID自动生成、HTTP服务器自动启动等功能
 */

use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::http_auto_config::{HTTPAutoConfig, HTTPAutoConfigOptions, HTTPConfig};
use crate::did_auto_config::{DIDAutoConfig, DIDAutoConfigOptions, DIDConfig, AgentInterface};
use crate::diap_key_generator::KeyType;
use crate::ipfs_registry::{IpfsRegistry, IpfsRegistryConfig, AgentRegistryEntry};

// 类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfigOptions {
    /// 是否自动启动HTTP服务器
    pub auto_start: Option<bool>,
    /// 是否自动分配端口
    pub auto_port: Option<bool>,
    /// 是否自动生成DID
    pub auto_did: Option<bool>,
    /// 发现服务地址
    pub discovery_service: Option<String>,
    /// 是否自动注册到 IPFS
    pub auto_ipfs_register: Option<bool>,
    /// IPFS 注册表配置
    pub ipfs_config: Option<IpfsRegistryConfig>,
    /// 端口范围
    pub port_range: Option<(u16, u16)>,
    /// 智能体名称
    pub agent_name: Option<String>,
    /// 自定义接口配置
    pub interfaces: Option<Vec<AgentInterface>>,
    /// 日志级别
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub did: String,
    pub did_web: Option<String>,
    pub port: u16,
    pub endpoint: String,
    pub local_ip: String,
    pub private_key: String,
    pub did_document: serde_json::Value,
    pub agent_description: serde_json::Value,
    pub ipfs_cid: Option<String>, // IPFS 注册表 CID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPRequest {
    pub content: Option<String>,
    pub message: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPResponse {
    pub response: String,
    pub timestamp: String,
    pub did: String,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/**
 * 自动配置DIAP智能体结构体
 */
pub struct AutoConfigAgent {
    options: AutoConfigOptions,
    http_config: Option<HTTPAutoConfig>,
    did_config: Option<DIDAutoConfig>,
    is_running: Arc<RwLock<bool>>,
}

impl AutoConfigAgent {
    /// 创建新的自动配置智能体
    pub fn new(options: AutoConfigOptions) -> Self {
        Self {
            options,
            http_config: None,
            did_config: None,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 核心方法：自动配置所有内容
    pub async fn auto_setup(&mut self) -> Result<AgentConfig> {
        info!("🔄 DIAP SDK: 开始自动配置...");
        
        // 步骤1: 配置HTTP服务器
        let http_setup = self.setup_http_server().await?;
        info!("✅ HTTP服务器配置完成: {}", http_setup.endpoint);
        
        // 步骤2: 配置DID文档
        let did_setup = self.setup_did_config(&http_setup).await?;
        info!("✅ DID配置完成: {}", did_setup.did);
        
        // 步骤3: 注册到发现服务
        self.register_to_discovery(&http_setup, &did_setup).await?;
        info!("✅ 注册到发现服务");
        
        // 步骤4: 注册到 IPFS（如果启用）
        let ipfs_cid = if self.options.auto_ipfs_register.unwrap_or(false) {
            match self.register_to_ipfs(&http_setup, &did_setup).await {
                Ok(cid) => {
                    info!("✅ IPFS 注册完成: {}", cid);
                    Some(cid)
                }
                Err(e) => {
                    warn!("⚠️ IPFS 注册失败: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        *self.is_running.write().await = true;
        info!("🎉 DIAP SDK: 自动配置完成！");
        
        Ok(self.get_config_with_ipfs(&http_setup, &did_setup, ipfs_cid).await)
    }

    /// 设置HTTP服务器
    async fn setup_http_server(&mut self) -> Result<HTTPConfig> {
        let http_options = HTTPAutoConfigOptions {
            auto_start: self.options.auto_start,
            auto_port: self.options.auto_port,
            port_range: self.options.port_range,
            host: Some("127.0.0.1".to_string()),
            log_level: self.options.log_level.clone(),
            routes: Some(Vec::new()),
        };

        let mut http_config = HTTPAutoConfig::new(http_options);
        let http_setup = http_config.auto_setup().await?;
        
        // 添加DIAP路由
        self.add_diap_routes(&mut http_config, &http_setup).await?;
        
        self.http_config = Some(http_config);
        Ok(http_setup)
    }

    /// 设置DID配置
    async fn setup_did_config(&mut self, http_setup: &HTTPConfig) -> Result<DIDConfig> {
        let did_options = DIDAutoConfigOptions {
            auto_did: self.options.auto_did,
            key_type: Some(KeyType::Ed25519),
            agent_name: self.options.agent_name.clone(),
            agent_description: Some("Automatically configured DIAP agent via Rust SDK".to_string()),
            agent_version: Some("1.0.0".to_string()),
            interfaces: self.options.interfaces.clone(),
            service_endpoints: Some(vec![
                crate::did_auto_config::ServiceEndpoint {
                    id: "diap-service".to_string(),
                    endpoint_type: "DIAPAgentService".to_string(),
                    service_endpoint: format!("{}/diap/api", http_setup.endpoint),
                    description: Some("Main DIAP communication endpoint".to_string()),
                }
            ]),
            log_level: self.options.log_level.clone(),
        };

        let mut did_config = DIDAutoConfig::new(did_options);
        let did_setup = did_config.auto_setup(&http_setup.local_ip, Some(http_setup.port)).await?;
        
        // 将 DID 文档和 AD 文档设置到 HTTP 服务器
        if let Some(ref http_config) = self.http_config {
            let did_doc_json = serde_json::to_value(&did_setup.did_document)?;
            let ad_doc_json = serde_json::to_value(&did_setup.agent_description)?;
            
            http_config.set_did_document(did_doc_json).await;
            http_config.set_ad_document(ad_doc_json).await;
            info!("✅ DID 和 AD 文档已设置到 HTTP 服务器");
        }
        
        self.did_config = Some(did_config);
        Ok(did_setup)
    }

    /// 添加DIAP路由
    async fn add_diap_routes(&self, http_config: &mut HTTPAutoConfig, _http_setup: &HTTPConfig) -> Result<()> {
        // DID文档端点
        http_config.add_route(crate::http_auto_config::RouteConfig {
            method: "GET".to_string(),
            path: "/.well-known/did.json".to_string(),
            handler_type: "json".to_string(),
        }).await;

        // 智能体描述文档端点
        http_config.add_route(crate::http_auto_config::RouteConfig {
            method: "GET".to_string(),
            path: "/agents/auto-agent/ad.json".to_string(),
            handler_type: "json".to_string(),
        }).await;

        // DIAP通信端点
        http_config.add_route(crate::http_auto_config::RouteConfig {
            method: "POST".to_string(),
            path: "/diap/api".to_string(),
            handler_type: "json".to_string(),
        }).await;

        Ok(())
    }

    /// 注册到发现服务
    async fn register_to_discovery(&self, http_setup: &HTTPConfig, did_setup: &DIDConfig) -> Result<()> {
        if let Some(ref discovery_service) = self.options.discovery_service {
            let client = reqwest::Client::new();
            let payload = serde_json::json!({
                "agent": did_setup.agent_description,
                "endpoint": http_setup.endpoint
            });

            match client.post(discovery_service)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("✅ 成功注册到发现服务");
                    } else {
                        warn!("⚠️ 发现服务注册失败: {}", response.status());
                    }
                }
                Err(e) => {
                    warn!("⚠️ 发现服务不可用: {}", e);
                }
            }
        } else {
            info!("⚠️ 未配置发现服务，跳过注册");
        }

        Ok(())
    }

    /// 注册到 IPFS
    async fn register_to_ipfs(&self, http_setup: &HTTPConfig, did_setup: &DIDConfig) -> Result<String> {
        let ipfs_config = self.options.ipfs_config.clone()
            .unwrap_or_default();
        
        let registry = IpfsRegistry::new(ipfs_config);
        
        // 提取能力和接口
        let capabilities: Vec<String> = did_setup.agent_description.capabilities
            .iter()
            .map(|c| c.name.clone())
            .collect();
        
        let interfaces: Vec<String> = did_setup.agent_description.interfaces
            .iter()
            .map(|i| i.interface_type.clone())
            .collect();
        
        let entry = AgentRegistryEntry {
            did: did_setup.did.clone(),
            did_web: did_setup.did_web.clone(),
            name: did_setup.agent_description.name.clone(),
            endpoint: http_setup.endpoint.clone(),
            did_document_url: format!("{}/.well-known/did.json", http_setup.endpoint),
            ad_url: format!("{}/agents/auto-agent/ad.json", http_setup.endpoint),
            capabilities,
            interfaces,
            registered_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        registry.publish_agent(entry).await
    }

    /// 获取配置信息
    pub async fn get_config(&self, http_setup: &HTTPConfig, did_setup: &DIDConfig) -> AgentConfig {
        self.get_config_with_ipfs(http_setup, did_setup, None).await
    }
    
    /// 获取配置信息（包含 IPFS CID）
    pub async fn get_config_with_ipfs(
        &self,
        http_setup: &HTTPConfig,
        did_setup: &DIDConfig,
        ipfs_cid: Option<String>,
    ) -> AgentConfig {
        AgentConfig {
            did: did_setup.did.clone(),
            did_web: did_setup.did_web.clone(),
            port: http_setup.port,
            endpoint: http_setup.endpoint.clone(),
            local_ip: http_setup.local_ip.clone(),
            private_key: did_setup.private_key.clone(),
            did_document: serde_json::to_value(&did_setup.did_document).unwrap_or_default(),
            agent_description: serde_json::to_value(&did_setup.agent_description).unwrap_or_default(),
            ipfs_cid,
        }
    }

    /// 获取服务端点
    pub fn get_endpoint(&self) -> Result<String> {
        if let Some(ref http_config) = self.http_config {
            http_config.get_endpoint()
        } else {
            Err(anyhow::anyhow!("Agent not configured yet"))
        }
    }

    /// 停止服务
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut http_config) = self.http_config {
            http_config.stop().await?;
        }
        *self.is_running.write().await = false;
        info!("🛑 DIAP Agent 已停止");
        Ok(())
    }

    /// 检查是否正在运行
    pub async fn is_agent_running(&self) -> bool {
        *self.is_running.read().await
    }
}

/**
 * DIAP客户端结构体
 */
pub struct DIAPClient {
    did: String,
    #[allow(dead_code)]
    private_key: String,
    client: reqwest::Client,
}

impl DIAPClient {
    /// 创建新的DIAP客户端
    pub fn new(did: String, private_key: String) -> Self {
        Self {
            did,
            private_key,
            client: reqwest::Client::new(),
        }
    }

    /// 发送请求到其他智能体
    pub async fn send_request(&self, target_url: &str, message: DIAPRequest) -> Result<DIAPResponse> {
        let signature = self.generate_signature(&message);
        
        let response = self.client
            .post(target_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("DIDWba did=\"{}\", signature=\"{}\"", self.did, signature))
            .json(&message)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP {}: {}", response.status(), response.status()));
        }
        
        let diap_response: DIAPResponse = response.json().await?;
        Ok(diap_response)
    }

    /// 生成签名（简化版）
    fn generate_signature(&self, _data: &DIAPRequest) -> String {
        // 这里应该实现完整的DID签名
        // 为了演示，返回一个模拟签名
        format!("mock_signature_{}", chrono::Utc::now().timestamp())
    }
}

/**
 * DIAP SDK主结构体
 */
pub struct DIAPSDK {
    options: AutoConfigOptions,
    agent: Option<AutoConfigAgent>,
    is_running: Arc<RwLock<bool>>,
}

impl DIAPSDK {
    /// 创建新的DIAP SDK实例
    pub fn new(options: AutoConfigOptions) -> Self {
        Self {
            options,
            agent: None,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 主要API：一键启动智能体
    pub async fn start(&mut self) -> Result<AgentConfig> {
        if *self.is_running.read().await {
            return Err(anyhow::anyhow!("Agent is already running"));
        }

        let mut agent = AutoConfigAgent::new(self.options.clone());
        let config = agent.auto_setup().await?;
        self.agent = Some(agent);
        *self.is_running.write().await = true;
        
        Ok(config)
    }

    /// 停止智能体
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut agent) = self.agent {
            agent.stop().await?;
        }
        self.agent = None;
        *self.is_running.write().await = false;
        Ok(())
    }

    /// 创建客户端
    pub fn create_client(&self, did: String, private_key: String) -> DIAPClient {
        DIAPClient::new(did, private_key)
    }

    /// 检查是否正在运行
    pub async fn is_agent_running(&self) -> bool {
        *self.is_running.read().await
    }
}

impl Default for AutoConfigOptions {
    fn default() -> Self {
        Self {
            auto_start: Some(true),
            auto_port: Some(true),
            auto_did: Some(true),
            discovery_service: None,
            auto_ipfs_register: Some(false), // 默认关闭，需要本地 IPFS 节点
            ipfs_config: None,
            port_range: Some((3000, 4000)),
            agent_name: Some("Auto-Configured DIAP Agent".to_string()),
            interfaces: Some(vec![AgentInterface {
                interface_type: "NaturalLanguageInterface".to_string(),
                description: "Auto-configured natural language interface".to_string(),
                url: None,
            }]),
            log_level: Some("info".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auto_config_agent() {
        let options = AutoConfigOptions::default();
        let mut agent = AutoConfigAgent::new(options);
        
        let result = agent.auto_setup().await;
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert!(config.did.starts_with("did:wba:"));
        assert!(config.port > 0);
        assert!(!config.endpoint.is_empty());
        
        agent.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_diap_sdk() {
        let options = AutoConfigOptions::default();
        let mut sdk = DIAPSDK::new(options);
        
        let result = sdk.start().await;
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert!(config.did.starts_with("did:wba:"));
        assert!(sdk.is_agent_running().await);
        
        sdk.stop().await.unwrap();
        assert!(!sdk.is_agent_running().await);
    }

    #[tokio::test]
    async fn test_diap_client() {
        let client = DIAPClient::new("did:wba:test".to_string(), "test_key".to_string());
        
        let request = DIAPRequest {
            content: Some("Hello".to_string()),
            message: None,
            extra: std::collections::HashMap::new(),
        };
        
        // 注意：这个测试会失败，因为没有真实的服务器
        // 在实际使用中，需要先启动一个DIAP智能体
        let result = client.send_request("http://localhost:3000/diap/api", request).await;
        assert!(result.is_err()); // 预期失败，因为没有服务器
    }
}

