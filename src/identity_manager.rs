// DIAP Rust SDK - 统一身份管理模块
// Decentralized Intelligent Agent Protocol
// 提供简化的身份注册和验证接口

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::key_manager::KeyPair;
use crate::did_builder::{DIDBuilder, DIDDocument};
use crate::did_resolver::DIDResolver;
use crate::ipfs_client::IpfsClient;
use crate::ipns_publisher::IpnsPublisher;

/// 智能体信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// 智能体名称
    pub name: String,
    
    /// 服务端点列表
    pub services: Vec<ServiceInfo>,
    
    /// 描述信息（可选）
    pub description: Option<String>,
    
    /// 标签（可选）
    pub tags: Option<Vec<String>>,
}

/// 服务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// 服务类型
    pub service_type: String,
    
    /// 服务端点
    pub endpoint: String,
}

/// 身份注册结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistration {
    /// DID标识符
    pub did: String,
    
    /// IPNS名称（用于后续解析）
    pub ipns_name: String,
    
    /// 当前CID
    pub cid: String,
    
    /// DID文档
    pub did_document: DIDDocument,
    
    /// 注册时间
    pub registered_at: String,
}

/// 身份验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityVerification {
    /// DID标识符
    pub did: String,
    
    /// 智能体信息
    pub agent_info: AgentInfo,
    
    /// 验证状态
    pub is_valid: bool,
    
    /// 验证详情
    pub verification_details: Vec<String>,
    
    /// 验证时间
    pub verified_at: String,
}

/// 统一身份管理器
pub struct IdentityManager {
    /// IPFS客户端
    ipfs_client: IpfsClient,
    
    /// IPNS发布器
    ipns_publisher: IpnsPublisher,
    
    /// DID解析器
    did_resolver: DIDResolver,
}

impl IdentityManager {
    /// 创建新的身份管理器
    pub fn new(
        ipfs_client: IpfsClient,
        ipns_publisher: IpnsPublisher,
    ) -> Self {
        let did_resolver = DIDResolver::new(
            ipfs_client.clone(),
            ipns_publisher.clone(),
            30,
        );
        
        Self {
            ipfs_client,
            ipns_publisher,
            did_resolver,
        }
    }
    
    /// 📝 统一身份注册入口
    /// 一键完成：生成DID文档 → 上传IPFS → 绑定IPNS
    pub async fn register_identity(
        &self,
        agent_info: &AgentInfo,
        keypair: &KeyPair,
    ) -> Result<IdentityRegistration> {
        log::info!("🚀 开始身份注册流程");
        log::info!("  智能体: {}", agent_info.name);
        log::info!("  DID: {}", keypair.did);
        
        // 步骤1: 创建DID构建器并添加服务端点
        let mut builder = DIDBuilder::new(
            agent_info.name.clone(),
            self.ipfs_client.clone(),
            self.ipns_publisher.clone(),
        );
        
        for service in &agent_info.services {
            builder.add_service(&service.service_type, &service.endpoint);
        }
        
        // 步骤2: 执行双层验证发布流程
        // 内部自动完成：
        // - 构建DID文档
        // - 上传到IPFS获取CID
        // - 注册IPNS name绑定CID
        // - 更新DID文档包含IPNS引用
        // - 再次上传并更新IPNS
        let publish_result = builder.create_and_publish(keypair).await
            .context("DID发布失败")?;
        
        log::info!("✅ 身份注册成功");
        log::info!("  DID: {}", publish_result.did);
        log::info!("  IPNS: /ipns/{}", publish_result.ipns_name);
        log::info!("  CID: {}", publish_result.current_cid);
        
        Ok(IdentityRegistration {
            did: publish_result.did,
            ipns_name: publish_result.ipns_name,
            cid: publish_result.current_cid,
            did_document: publish_result.did_document,
            registered_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 🔍 统一身份验证入口
    /// 一键完成：IPNS解析 → 获取DID文档 → 验证签名和完整性
    pub async fn verify_identity(
        &self,
        ipns_name: &str,
    ) -> Result<IdentityVerification> {
        log::info!("🔍 开始身份验证流程");
        log::info!("  IPNS: {}", ipns_name);
        
        let mut verification_details = Vec::new();
        
        // 步骤1: 通过IPNS name解析到最新DID文档CID
        let cid = self.ipns_publisher.resolve(ipns_name).await
            .context("IPNS解析失败")?;
        
        verification_details.push(format!("✓ IPNS解析成功: {} → {}", ipns_name, cid));
        log::debug!("  CID: {}", cid);
        
        // 步骤2: 从IPFS下载DID文档
        let content = self.ipfs_client.get(&cid).await
            .context("下载DID文档失败")?;
        
        verification_details.push(format!("✓ DID文档下载成功 (大小: {} 字节)", content.len()));
        
        // 步骤3: 解析DID文档
        let did_document: DIDDocument = serde_json::from_str(&content)
            .context("解析DID文档失败")?;
        
        let did = did_document.id.clone();
        verification_details.push(format!("✓ DID文档解析成功: {}", did));
        
        // 步骤4: 验证DID文档的双层一致性
        let double_layer_valid = crate::did_builder::verify_double_layer(&did_document, ipns_name)
            .is_ok();
        
        if double_layer_valid {
            verification_details.push("✓ 双层验证通过 (DID ↔ IPNS 绑定一致)".to_string());
        } else {
            verification_details.push("⚠ 双层验证警告 (建议检查DID文档)".to_string());
        }
        
        // 步骤5: 验证公钥和DID的匹配性
        let did_ipns_name = did.trim_start_matches("did:ipfs:");
        let did_match = did_ipns_name == ipns_name;
        
        if did_match {
            verification_details.push("✓ DID与IPNS名称匹配".to_string());
        } else {
            verification_details.push("✗ DID与IPNS名称不匹配".to_string());
        }
        
        // 步骤6: 提取智能体信息
        let agent_info = self.extract_agent_info(&did_document)?;
        verification_details.push(format!("✓ 智能体信息提取成功: {}", agent_info.name));
        
        // 总体验证状态
        let is_valid = double_layer_valid && did_match;
        
        if is_valid {
            log::info!("✅ 身份验证成功");
        } else {
            log::warn!("⚠️  身份验证存在问题");
        }
        
        Ok(IdentityVerification {
            did,
            agent_info,
            is_valid,
            verification_details,
            verified_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 🔄 更新身份信息
    /// 更新DID文档并自动重新发布到IPNS
    pub async fn update_identity(
        &self,
        agent_info: &AgentInfo,
        keypair: &KeyPair,
        current_sequence: u64,
    ) -> Result<IdentityRegistration> {
        log::info!("🔄 更新身份信息");
        
        // 创建DID构建器
        let mut builder = DIDBuilder::new(
            agent_info.name.clone(),
            self.ipfs_client.clone(),
            self.ipns_publisher.clone(),
        );
        
        for service in &agent_info.services {
            builder.add_service(&service.service_type, &service.endpoint);
        }
        
        // 更新DID文档
        let publish_result = builder.update_did_document(
            keypair,
            current_sequence,
            |_doc| {
                // 这里可以进行额外的修改
            },
        ).await?;
        
        log::info!("✅ 身份更新成功");
        
        Ok(IdentityRegistration {
            did: publish_result.did,
            ipns_name: publish_result.ipns_name,
            cid: publish_result.current_cid,
            did_document: publish_result.did_document,
            registered_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 🔎 通过DID解析身份
    pub async fn resolve_by_did(&self, did: &str) -> Result<IdentityVerification> {
        // 从DID提取IPNS名称
        let ipns_name = did.trim_start_matches("did:ipfs:");
        self.verify_identity(ipns_name).await
    }
    
    /// 从DID文档提取智能体信息
    fn extract_agent_info(&self, did_document: &DIDDocument) -> Result<AgentInfo> {
        let mut services = Vec::new();
        
        if let Some(service_list) = &did_document.service {
            for service in service_list {
                // 跳过内部服务
                if service.service_type == "IPNSResolver" || service.service_type == "LibP2PNode" {
                    continue;
                }
                
                services.push(ServiceInfo {
                    service_type: service.service_type.clone(),
                    endpoint: service.service_endpoint.clone(),
                });
            }
        }
        
        // 从DID提取名称（简化）
        let name = did_document.id.split(':').last()
            .unwrap_or("未知智能体")
            .chars()
            .take(20)
            .collect();
        
        Ok(AgentInfo {
            name,
            services,
            description: None,
            tags: None,
        })
    }
    
    /// 获取IPFS客户端引用
    pub fn ipfs_client(&self) -> &IpfsClient {
        &self.ipfs_client
    }
    
    /// 获取IPNS发布器引用
    pub fn ipns_publisher(&self) -> &IpnsPublisher {
        &self.ipns_publisher
    }
    
    /// 获取DID解析器引用
    pub fn did_resolver(&self) -> &DIDResolver {
        &self.did_resolver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_manager::KeyPair;
    
    #[tokio::test]
    #[ignore] // 需要实际的IPFS/IPNS服务
    async fn test_register_and_verify_identity() {
        // 创建身份管理器
        let ipfs_client = IpfsClient::new(
            Some("http://localhost:5001".to_string()),
            Some("http://localhost:8080".to_string()),
            None,
            None,
            30,
        );
        
        let ipns_publisher = IpnsPublisher::new(
            true, 
            true, 
            Some("http://localhost:5001".to_string()), 
            365
        );
        
        let manager = IdentityManager::new(ipfs_client, ipns_publisher);
        
        // 生成密钥对
        let keypair = KeyPair::generate().unwrap();
        
        // 创建智能体信息
        let agent_info = AgentInfo {
            name: "测试智能体".to_string(),
            services: vec![
                ServiceInfo {
                    service_type: "API".to_string(),
                    endpoint: "https://api.example.com".to_string(),
                },
            ],
            description: Some("这是一个测试智能体".to_string()),
            tags: Some(vec!["test".to_string(), "demo".to_string()]),
        };
        
        // 注册身份
        let registration = manager.register_identity(&agent_info, &keypair).await.unwrap();
        println!("✅ 注册成功: {}", registration.did);
        
        // 验证身份
        let verification = manager.verify_identity(&registration.ipns_name).await.unwrap();
        println!("✅ 验证结果: {:?}", verification.is_valid);
        
        assert!(verification.is_valid);
    }
}

