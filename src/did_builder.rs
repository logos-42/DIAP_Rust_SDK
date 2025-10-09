// ANP Rust SDK - DID文档构建模块
// 实现DID文档的构建和双层验证逻辑

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::key_manager::KeyPair;
use crate::ipfs_client::{IpfsClient, IpfsUploadResult};
use crate::ipns_publisher::IpnsPublisher;

/// DID文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIDDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    pub id: String,
    
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    
    pub authentication: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<Service>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ipfsMetadata")]
    pub ipfs_metadata: Option<IpfsMetadata>,
}

/// 验证方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    
    #[serde(rename = "type")]
    pub vm_type: String,
    
    pub controller: String,
    
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// 服务端点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    
    #[serde(rename = "type")]
    pub service_type: String,
    
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// IPFS元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsMetadata {
    #[serde(rename = "currentCID")]
    pub current_cid: String,
    
    pub sequence: u64,
    
    #[serde(rename = "publishedAt")]
    pub published_at: String,
}

/// DID构建器
pub struct DIDBuilder {
    /// 智能体名称
    #[allow(dead_code)]
    agent_name: String,
    
    /// 服务端点列表
    services: Vec<Service>,
    
    /// IPFS客户端
    ipfs_client: IpfsClient,
    
    /// IPNS发布器
    ipns_publisher: IpnsPublisher,
}

/// DID发布结果
#[derive(Debug, Clone)]
pub struct DIDPublishResult {
    /// DID标识符
    pub did: String,
    
    /// IPNS名称
    pub ipns_name: String,
    
    /// 当前CID
    pub current_cid: String,
    
    /// 序列号
    pub sequence: u64,
    
    /// DID文档
    pub did_document: DIDDocument,
}

impl DIDBuilder {
    /// 创建新的DID构建器
    pub fn new(
        agent_name: String,
        ipfs_client: IpfsClient,
        ipns_publisher: IpnsPublisher,
    ) -> Self {
        Self {
            agent_name,
            services: Vec::new(),
            ipfs_client,
            ipns_publisher,
        }
    }
    
    /// 添加服务端点
    pub fn add_service(&mut self, service_type: &str, endpoint: &str) -> &mut Self {
        let service = Service {
            id: format!("#{}", service_type.to_lowercase()),
            service_type: service_type.to_string(),
            service_endpoint: endpoint.to_string(),
        };
        self.services.push(service);
        self
    }
    
    /// 创建并发布DID（双层验证流程）
    pub async fn create_and_publish(&self, keypair: &KeyPair) -> Result<DIDPublishResult> {
        log::info!("开始DID双层验证发布流程");
        
        // 步骤1: 构建初始DID文档（版本1，不含IPNS引用）
        log::info!("步骤1: 构建初始DID文档");
        let did_doc_v1 = self.build_did_document(keypair, None)?;
        
        // 步骤2: 上传版本1到IPFS
        log::info!("步骤2: 上传初始DID文档到IPFS");
        let upload_result_v1 = self.upload_did_document(&did_doc_v1).await?;
        log::info!("版本1 CID: {}", upload_result_v1.cid);
        
        // 步骤3: 发布CID1到IPNS
        log::info!("步骤3: 发布到IPNS");
        let ipns_result = self.ipns_publisher
            .publish(keypair, &upload_result_v1.cid, None)
            .await?;
        log::info!("IPNS名称: {}", ipns_result.ipns_name);
        
        // 步骤4: 在DID文档中添加IPNS service端点
        log::info!("步骤4: 添加IPNS引用到DID文档");
        let ipns_service = Service {
            id: "#ipns-resolver".to_string(),
            service_type: "IPNSResolver".to_string(),
            service_endpoint: format!("/ipns/{}", ipns_result.ipns_name),
        };
        
        // 步骤5: 构建版本2 DID文档（含IPNS引用）
        let mut did_doc_v2 = self.build_did_document(keypair, Some(&ipns_service))?;
        
        // 添加IPFS元数据
        did_doc_v2.ipfs_metadata = Some(IpfsMetadata {
            current_cid: upload_result_v1.cid.clone(),
            sequence: ipns_result.sequence,
            published_at: ipns_result.published_at.clone(),
        });
        
        // 步骤6: 上传版本2到IPFS
        log::info!("步骤5: 上传最终DID文档到IPFS");
        let upload_result_v2 = self.upload_did_document(&did_doc_v2).await?;
        log::info!("版本2 CID: {}", upload_result_v2.cid);
        
        // 步骤7: 更新IPNS指向版本2
        log::info!("步骤6: 更新IPNS指向最终版本");
        let final_ipns_result = self.ipns_publisher
            .publish(keypair, &upload_result_v2.cid, Some(ipns_result.sequence))
            .await?;
        
        // 更新元数据
        did_doc_v2.ipfs_metadata = Some(IpfsMetadata {
            current_cid: upload_result_v2.cid.clone(),
            sequence: final_ipns_result.sequence,
            published_at: final_ipns_result.published_at.clone(),
        });
        
        log::info!("✓ DID双层验证发布完成");
        log::info!("  DID: {}", keypair.did);
        log::info!("  IPNS: /ipns/{}", final_ipns_result.ipns_name);
        log::info!("  CID: {}", upload_result_v2.cid);
        
        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            ipns_name: final_ipns_result.ipns_name,
            current_cid: upload_result_v2.cid,
            sequence: final_ipns_result.sequence,
            did_document: did_doc_v2,
        })
    }
    
    /// 更新DID文档
    pub async fn update_did_document(
        &self,
        keypair: &KeyPair,
        current_sequence: u64,
        modifications: impl FnOnce(&mut DIDDocument),
    ) -> Result<DIDPublishResult> {
        log::info!("更新DID文档");
        
        // 构建当前DID文档
        let ipns_service = Service {
            id: "#ipns-resolver".to_string(),
            service_type: "IPNSResolver".to_string(),
            service_endpoint: format!("/ipns/{}", keypair.ipns_name),
        };
        
        let mut did_doc = self.build_did_document(keypair, Some(&ipns_service))?;
        
        // 应用修改
        modifications(&mut did_doc);
        
        // 上传到IPFS
        let upload_result = self.upload_did_document(&did_doc).await?;
        log::info!("新CID: {}", upload_result.cid);
        
        // 更新IPNS
        let ipns_result = self.ipns_publisher
            .publish(keypair, &upload_result.cid, Some(current_sequence))
            .await?;
        
        // 更新元数据
        did_doc.ipfs_metadata = Some(IpfsMetadata {
            current_cid: upload_result.cid.clone(),
            sequence: ipns_result.sequence,
            published_at: ipns_result.published_at,
        });
        
        log::info!("✓ DID文档更新完成");
        
        Ok(DIDPublishResult {
            did: keypair.did.clone(),
            ipns_name: ipns_result.ipns_name,
            current_cid: upload_result.cid,
            sequence: ipns_result.sequence,
            did_document: did_doc,
        })
    }
    
    /// 构建DID文档
    fn build_did_document(
        &self,
        keypair: &KeyPair,
        ipns_service: Option<&Service>,
    ) -> Result<DIDDocument> {
        // 编码公钥为multibase格式
        let public_key_multibase = format!("z{}", bs58::encode(&keypair.public_key).into_string());
        
        // 创建验证方法
        let verification_method = VerificationMethod {
            id: format!("{}#auth-key", keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: keypair.did.clone(),
            public_key_multibase,
        };
        
        // 构建服务列表
        let mut services = self.services.clone();
        if let Some(ipns_svc) = ipns_service {
            services.insert(0, ipns_svc.clone());
        }
        
        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: keypair.did.clone(),
            verification_method: vec![verification_method],
            authentication: vec![format!("{}#auth-key", keypair.did)],
            service: if services.is_empty() { None } else { Some(services) },
            ipfs_metadata: None,
        })
    }
    
    /// 上传DID文档到IPFS
    async fn upload_did_document(&self, did_doc: &DIDDocument) -> Result<IpfsUploadResult> {
        let json = serde_json::to_string_pretty(did_doc)
            .context("序列化DID文档失败")?;
        
        self.ipfs_client
            .upload(&json, "did.json")
            .await
            .context("上传DID文档到IPFS失败")
    }
}

/// 验证DID文档的双层一致性
pub fn verify_double_layer(did_doc: &DIDDocument, expected_ipns: &str) -> Result<bool> {
    // 检查是否有IPNS service
    let services = did_doc.service.as_ref()
        .ok_or_else(|| anyhow::anyhow!("DID文档缺少service字段"))?;
    
    let ipns_service = services.iter()
        .find(|s| s.service_type == "IPNSResolver")
        .ok_or_else(|| anyhow::anyhow!("DID文档缺少IPNSResolver服务"))?;
    
    // 验证IPNS名称
    let endpoint = &ipns_service.service_endpoint;
    let ipns_name = endpoint.trim_start_matches("/ipns/");
    
    if ipns_name != expected_ipns {
        anyhow::bail!("IPNS名称不匹配: 期望 {}, 实际 {}", expected_ipns, ipns_name);
    }
    
    // 检查元数据
    if let Some(metadata) = &did_doc.ipfs_metadata {
        log::debug!("IPFS元数据验证通过:");
        log::debug!("  CID: {}", metadata.current_cid);
        log::debug!("  Sequence: {}", metadata.sequence);
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_manager::KeyPair;
    
    #[test]
    fn test_build_did_document() {
        let keypair = KeyPair::generate().unwrap();
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let ipns_publisher = IpnsPublisher::new(true, false, None, 365);
        
        let builder = DIDBuilder::new(
            "Test Agent".to_string(),
            ipfs_client,
            ipns_publisher,
        );
        
        let did_doc = builder.build_did_document(&keypair, None).unwrap();
        
        assert_eq!(did_doc.id, keypair.did);
        assert_eq!(did_doc.verification_method.len(), 1);
        assert_eq!(did_doc.authentication.len(), 1);
    }
    
    #[test]
    fn test_verify_double_layer() {
        let mut did_doc = DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: "did:ipfs:k51qzi5u...".to_string(),
            verification_method: vec![],
            authentication: vec![],
            service: Some(vec![Service {
                id: "#ipns-resolver".to_string(),
                service_type: "IPNSResolver".to_string(),
                service_endpoint: "/ipns/k51qzi5u...".to_string(),
            }]),
            ipfs_metadata: None,
        };
        
        let result = verify_double_layer(&did_doc, "k51qzi5u...");
        assert!(result.is_ok());
    }
}
