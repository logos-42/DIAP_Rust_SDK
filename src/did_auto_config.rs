/**
 * DIAP DID文档自动配置模块 - Rust版本
 * 提供DID自动生成、DID文档自动配置等功能
 */

use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::info;
use chrono::Utc;

use crate::diap_key_generator::{DIAPKeyGenerator, KeyType, DIDDocument, Service};

// 类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDAutoConfigOptions {
    /// 是否自动生成DID
    pub auto_did: Option<bool>,
    /// 密钥类型
    pub key_type: Option<KeyType>,
    /// 智能体名称
    pub agent_name: Option<String>,
    /// 智能体描述
    pub agent_description: Option<String>,
    /// 智能体版本
    pub agent_version: Option<String>,
    /// 自定义接口配置
    pub interfaces: Option<Vec<AgentInterface>>,
    /// 服务端点配置
    pub service_endpoints: Option<Vec<ServiceEndpoint>>,
    /// 日志级别
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInterface {
    pub interface_type: String, // "NaturalLanguageInterface" | "StructuredInterface"
    pub description: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    pub endpoint_type: String,
    pub service_endpoint: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDConfig {
    pub did: String,
    pub did_web: Option<String>, // did:web 格式
    pub private_key: String,
    pub public_key: String,
    pub did_document: DIDDocument,
    pub agent_description: AgentDescription,
    pub key_type: KeyType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDescription {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    #[serde(rename = "@type")]
    pub description_type: String,
    pub name: String,
    pub did: String,
    pub description: String,
    pub version: String,
    pub created: String,
    #[serde(rename = "ad:interfaces")]
    pub interfaces: Vec<InterfaceDescription>,
    #[serde(rename = "ad:capabilities")]
    pub capabilities: Vec<Capability>,
    #[serde(rename = "ad:supportedProtocols")]
    pub supported_protocols: Vec<Protocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDescription {
    #[serde(rename = "@type")]
    pub interface_type: String,
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    #[serde(rename = "@type")]
    pub capability_type: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protocol {
    #[serde(rename = "@type")]
    pub protocol_type: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

/**
 * DID文档自动配置结构体
 */
pub struct DIDAutoConfig {
    options: DIDAutoConfigOptions,
    auto_did: Option<String>,
    auto_did_web: Option<String>,
    private_key: Option<String>,
    public_key: Option<String>,
    did_document: Option<DIDDocument>,
    agent_description: Option<AgentDescription>,
}

impl DIDAutoConfig {
    /// 创建新的DID自动配置实例
    pub fn new(options: DIDAutoConfigOptions) -> Self {
        Self {
            options,
            auto_did: None,
            auto_did_web: None,
            private_key: None,
            public_key: None,
            did_document: None,
            agent_description: None,
        }
    }

    /// 核心方法：自动配置DID和文档
    pub async fn auto_setup(&mut self, domain: &str, port: Option<u16>) -> Result<DIDConfig> {
        info!("🔄 DID自动配置: 开始配置...");
        
        // 步骤1: 生成DID和密钥对
        self.generate_did_and_keys(domain, port).await?;
        info!("✅ 生成DID: {}", self.auto_did.as_ref().unwrap());
        
        // 步骤2: 配置DID文档
        self.configure_did_document().await?;
        info!("✅ DID文档配置完成");
        
        // 步骤3: 生成智能体描述文档
        self.generate_agent_description().await?;
        info!("✅ 智能体描述文档生成完成");
        
        info!("🎉 DID自动配置完成！");
        Ok(self.get_config()?)
    }

    /// 生成DID和密钥对
    async fn generate_did_and_keys(&mut self, domain: &str, port: Option<u16>) -> Result<()> {
        // 构建完整的域名（包含端口）
        let full_domain = if let Some(port) = port {
            format!("{}:{}", domain, port)
        } else {
            domain.to_string()
        };
        
        // 使用DIAP密钥生成器
        let generator = DIAPKeyGenerator::new(full_domain, Some("auto-agent".to_string()));
        let key_type = self.options.key_type.clone().unwrap_or(KeyType::Ed25519);
        let key_pair = generator.generate_keypair(key_type)?;
        
        self.auto_did = Some(key_pair.did);
        self.auto_did_web = key_pair.did_web;
        self.private_key = Some(key_pair.private_key);
        
        // 解析DID文档
        let did_doc: DIDDocument = serde_json::from_str(&key_pair.did_document)?;
        self.did_document = Some(did_doc);
        
        // 提取公钥
        if let Some(ref doc) = self.did_document {
            if let Some(ref methods) = doc.verification_method {
                if let Some(ref method) = methods.first() {
                    if let Some(ref multibase) = method.public_key_multibase {
                        self.public_key = Some(multibase.clone());
                    }
                }
            }
        }
        
        Ok(())
    }

    /// 配置DID文档
    async fn configure_did_document(&mut self) -> Result<()> {
        let did_doc = self.did_document.as_mut()
            .ok_or_else(|| anyhow::anyhow!("DID document not generated yet"))?;

        // 更新服务端点
        if let Some(ref service_endpoints) = self.options.service_endpoints {
            let services: Vec<Service> = service_endpoints.iter().map(|endpoint| {
                Service {
                    id: endpoint.id.clone(),
                    service_type: endpoint.endpoint_type.clone(),
                    service_endpoint: endpoint.service_endpoint.clone(),
                }
            }).collect();
            did_doc.service = Some(services);
        }

        // 确保必要的字段存在
        if did_doc.service.is_none() {
            did_doc.service = Some(Vec::new());
        }

        // 添加默认服务端点（如果存在）
        let has_default = self.options.service_endpoints.as_ref()
            .map(|eps| eps.iter().any(|ep| ep.id == "default"))
            .unwrap_or(false);
            
        if !has_default && self.options.service_endpoints.as_ref().map(|eps| eps.is_empty()).unwrap_or(true) {
            did_doc.service.as_mut().unwrap().push(Service {
                id: "default".to_string(),
                service_type: "DIAPAgentService".to_string(),
                service_endpoint: "http://localhost:3000/diap/api".to_string(),
            });
        }

        Ok(())
    }

    /// 生成智能体描述文档
    async fn generate_agent_description(&mut self) -> Result<()> {
        let empty_vec = Vec::new();
        let interfaces = self.options.interfaces.as_ref().unwrap_or(&empty_vec);
        let default_url = self.get_default_interface_url().await?;
        
        let interface_descriptions: Vec<InterfaceDescription> = interfaces.iter().map(|iface| {
            InterfaceDescription {
                interface_type: format!("ad:{}", iface.interface_type),
                url: iface.url.clone().unwrap_or_else(|| default_url.clone()),
                description: iface.description.clone(),
            }
        }).collect();

        let agent_desc = AgentDescription {
            context: serde_json::json!({
                "@vocab": "https://schema.org/",
                "ad": "https://service.agent-network-protocol.com/ad#"
            }),
            description_type: "ad:AgentDescription".to_string(),
            name: self.options.agent_name.as_ref().unwrap_or(&"Auto-Configured DIAP Agent".to_string()).clone(),
            did: self.auto_did.as_ref().unwrap().clone(),
            description: self.options.agent_description.as_ref().unwrap_or(&"Automatically configured DIAP agent via SDK".to_string()).clone(),
            version: self.options.agent_version.as_ref().unwrap_or(&"1.0.0".to_string()).clone(),
            created: Utc::now().to_rfc3339(),
            interfaces: interface_descriptions,
            capabilities: vec![
                Capability {
                    capability_type: "ad:Capability".to_string(),
                    name: "Natural Language Processing".to_string(),
                    description: "Process natural language requests and responses".to_string(),
                },
                Capability {
                    capability_type: "ad:Capability".to_string(),
                    name: "HTTP Communication".to_string(),
                    description: "Communicate via HTTP protocol".to_string(),
                }
            ],
            supported_protocols: vec![
                Protocol {
                    protocol_type: "ad:Protocol".to_string(),
                    name: "DIAP".to_string(),
                    version: "1.0".to_string(),
                    description: "Agent Network Protocol".to_string(),
                }
            ],
        };

        self.agent_description = Some(agent_desc);
        Ok(())
    }

    /// 获取默认接口URL
    async fn get_default_interface_url(&self) -> Result<String> {
        // 从DID文档中获取服务端点
        if let Some(ref doc) = self.did_document {
            if let Some(ref services) = doc.service {
                if let Some(ref service) = services.first() {
                    return Ok(service.service_endpoint.clone());
                }
            }
        }
        Ok("http://localhost:3000/diap/api".to_string())
    }

    /// 更新服务端点
    pub fn update_service_endpoint(&mut self, endpoint: ServiceEndpoint) -> Result<()> {
        let did_doc = self.did_document.as_mut()
            .ok_or_else(|| anyhow::anyhow!("DID document not configured yet"))?;

        if did_doc.service.is_none() {
            did_doc.service = Some(Vec::new());
        }

        let services = did_doc.service.as_mut().unwrap();
        
        // 查找现有端点并更新，或添加新端点
        if let Some(existing_index) = services.iter().position(|s| s.id == endpoint.id) {
            services[existing_index] = Service {
                id: endpoint.id.clone(),
                service_type: endpoint.endpoint_type.clone(),
                service_endpoint: endpoint.service_endpoint.clone(),
            };
        } else {
            services.push(Service {
                id: endpoint.id.clone(),
                service_type: endpoint.endpoint_type.clone(),
                service_endpoint: endpoint.service_endpoint.clone(),
            });
        }

        info!("✅ 更新服务端点: {}", endpoint.id);
        Ok(())
    }

    /// 添加接口
    pub async fn add_interface(&mut self, iface: AgentInterface) -> Result<()> {
        if let Some(ref mut interfaces) = self.options.interfaces {
            interfaces.push(iface.clone());
        } else {
            self.options.interfaces = Some(vec![iface.clone()]);
        }
        
        // 获取默认URL，避免借用冲突
        let default_url = self.get_default_interface_url().await?;
        
        if let Some(ref mut agent_desc) = self.agent_description {
            let interface_desc = InterfaceDescription {
                interface_type: format!("ad:{}", iface.interface_type),
                url: iface.url.unwrap_or(default_url),
                description: iface.description,
            };
            agent_desc.interfaces.push(interface_desc);
        }

        info!("✅ 添加接口: {}", iface.interface_type);
        Ok(())
    }

    /// 获取配置信息
    pub fn get_config(&self) -> Result<DIDConfig> {
        let did = self.auto_did.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID not configured yet"))?;
        let private_key = self.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Private key not configured yet"))?;
        let public_key = self.public_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Public key not configured yet"))?;
        let did_document = self.did_document.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID document not configured yet"))?;
        let agent_description = self.agent_description.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Agent description not configured yet"))?;

        Ok(DIDConfig {
            did: did.clone(),
            did_web: self.auto_did_web.clone(),
            private_key: private_key.clone(),
            public_key: public_key.clone(),
            did_document: did_document.clone(),
            agent_description: agent_description.clone(),
            key_type: self.options.key_type.clone().unwrap_or(KeyType::Ed25519),
        })
    }

    /// 获取DID
    pub fn get_did(&self) -> Result<String> {
        Ok(self.auto_did.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID not configured yet"))?.clone())
    }

    /// 获取私钥
    pub fn get_private_key(&self) -> Result<String> {
        Ok(self.private_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Private key not configured yet"))?.clone())
    }

    /// 获取公钥
    pub fn get_public_key(&self) -> Result<String> {
        Ok(self.public_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Public key not configured yet"))?.clone())
    }

    /// 获取DID文档
    pub fn get_did_document(&self) -> Result<&DIDDocument> {
        self.did_document.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DID document not configured yet"))
    }

    /// 获取智能体描述文档
    pub fn get_agent_description(&self) -> Result<&AgentDescription> {
        self.agent_description.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Agent description not configured yet"))
    }

    /// 导出DID文档为JSON字符串
    pub fn export_did_document(&self) -> Result<String> {
        let doc = self.get_did_document()?;
        Ok(serde_json::to_string_pretty(doc)?)
    }

    /// 导出智能体描述文档为JSON字符串
    pub fn export_agent_description(&self) -> Result<String> {
        let desc = self.get_agent_description()?;
        Ok(serde_json::to_string_pretty(desc)?)
    }

    /// 验证DID文档格式
    pub fn validate_did_document(&self) -> Result<bool> {
        let doc = self.get_did_document()?;
        
        // 基本验证
        if doc.context.is_empty() || doc.id.is_empty() || doc.verification_method.is_none() {
            return Ok(false);
        }

        // 验证DID格式
        if !doc.id.starts_with("did:wba:") {
            return Ok(false);
        }

        Ok(true)
    }
}

impl Default for DIDAutoConfigOptions {
    fn default() -> Self {
        Self {
            auto_did: Some(true),
            key_type: Some(KeyType::Ed25519),
            agent_name: Some("Auto-Configured DIAP Agent".to_string()),
            agent_description: Some("Automatically configured DIAP agent via SDK".to_string()),
            agent_version: Some("1.0.0".to_string()),
            interfaces: Some(vec![AgentInterface {
                interface_type: "NaturalLanguageInterface".to_string(),
                description: "Auto-configured natural language interface".to_string(),
                url: None,
            }]),
            service_endpoints: Some(Vec::new()),
            log_level: Some("info".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_did_auto_config() {
        let options = DIDAutoConfigOptions::default();
        let mut config = DIDAutoConfig::new(options);
        
        let result = config.auto_setup("localhost", Some(3000)).await;
        assert!(result.is_ok());
        
        let did_config = result.unwrap();
        assert!(did_config.did.starts_with("did:wba:"));
        assert!(!did_config.private_key.is_empty());
        assert!(!did_config.public_key.is_empty());
    }

    #[tokio::test]
    async fn test_did_validation() {
        let options = DIDAutoConfigOptions::default();
        let mut config = DIDAutoConfig::new(options);
        
        config.auto_setup("test.com", None).await.unwrap();
        assert!(config.validate_did_document().unwrap());
    }
}
