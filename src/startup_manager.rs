// DIAP Rust SDK - 启动管理器模块
// Decentralized Intelligent Agent Protocol
// 负责智能体启动时自动更新DID文档，包含最新的libp2p多地址

use anyhow::Result;
use crate::key_manager::KeyPair;
use crate::libp2p_identity::LibP2PIdentity;
use crate::libp2p_node::{LibP2PNode, NodeInfo};
use crate::ipfs_client::IpfsClient;
use crate::ipns_publisher::IpnsPublisher;
use crate::did_builder::{Service, DIDPublishResult};

/// 启动时更新的配置
pub struct StartupConfig {
    /// 是否每次启动都更新
    pub always_update: bool,
    
    /// 地址变化阈值（秒），超过此时间认为地址可能过期
    pub address_freshness_threshold: u64,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            always_update: true,  // 默认每次都更新
            address_freshness_threshold: 3600,  // 1小时
        }
    }
}

/// 启动管理器
pub struct StartupManager {
    /// IPNS密钥对（用于DID标识）
    ipns_keypair: KeyPair,
    
    /// libp2p身份（用于P2P通信）
    libp2p_identity: LibP2PIdentity,
    
    /// IPFS客户端
    ipfs_client: IpfsClient,
    
    /// IPNS发布器
    ipns_publisher: IpnsPublisher,
    
    /// 启动配置
    #[allow(dead_code)]
    config: StartupConfig,
}

impl StartupManager {
    /// 创建新的启动管理器
    pub fn new(
        ipns_keypair: KeyPair,
        libp2p_identity: LibP2PIdentity,
        ipfs_client: IpfsClient,
        ipns_publisher: IpnsPublisher,
        config: StartupConfig,
    ) -> Self {
        Self {
            ipns_keypair,
            libp2p_identity,
            ipfs_client,
            ipns_publisher,
            config,
        }
    }
    
    /// 启动时更新DID文档
    /// 包含最新的libp2p节点信息
    pub async fn update_on_startup(
        &self,
        node: &LibP2PNode,
        current_sequence: Option<u64>,
    ) -> Result<DIDPublishResult> {
        log::info!("启动时更新DID文档");
        
        // 获取最新的节点信息
        let node_info = node.get_node_info();
        
        log::info!("当前libp2p节点信息:");
        log::info!("  PeerID: {}", node_info.peer_id);
        log::info!("  多地址数量: {}", node_info.multiaddrs.len());
        for addr in &node_info.multiaddrs {
            log::info!("    - {}", addr);
        }
        
        // 注意：这里不使用DID构建器，直接构建文档
        // let _did_builder = DIDBuilder::new(...);
        
        // 构建包含libp2p信息的DID文档
        let did_doc = self.build_did_with_libp2p(&node_info)?;
        
        // 上传到IPFS
        let json = serde_json::to_string_pretty(&did_doc)?;
        let upload_result = self.ipfs_client.upload(&json, "did.json").await?;
        
        log::info!("DID文档已上传到IPFS");
        log::info!("  CID: {}", upload_result.cid);
        
        // 发布/更新IPNS
        let ipns_result = self.ipns_publisher.publish(
            &self.ipns_keypair,
            &upload_result.cid,
            current_sequence,
        ).await?;
        
        log::info!("IPNS已更新");
        log::info!("  IPNS名称: {}", ipns_result.ipns_name);
        log::info!("  序列号: {}", ipns_result.sequence);
        
        Ok(DIDPublishResult {
            did: self.ipns_keypair.did.clone(),
            ipns_name: ipns_result.ipns_name,
            current_cid: upload_result.cid,
            sequence: ipns_result.sequence,
            did_document: did_doc,
        })
    }
    
    /// 构建包含libp2p信息的DID文档
    fn build_did_with_libp2p(&self, node_info: &NodeInfo) -> Result<crate::did_builder::DIDDocument> {
        use crate::did_builder::{DIDDocument, VerificationMethod};
        
        // 创建IPNS密钥的验证方法
        let ipns_public_key_multibase = format!("z{}", bs58::encode(&self.ipns_keypair.public_key).into_string());
        
        let ipns_verification_method = VerificationMethod {
            id: format!("{}#ipns-key", self.ipns_keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: self.ipns_keypair.did.clone(),
            public_key_multibase: ipns_public_key_multibase,
        };
        
        // 创建libp2p密钥的验证方法
        let libp2p_verification_method = VerificationMethod {
            id: format!("{}#libp2p-key", self.ipns_keypair.did),
            vm_type: "Ed25519VerificationKey2020".to_string(),
            controller: self.ipns_keypair.did.clone(),
            public_key_multibase: self.libp2p_identity.public_key_multibase(),
        };
        
        // 创建IPNS resolver服务
        let ipns_service = Service {
            id: "#ipns-resolver".to_string(),
            service_type: "IPNSResolver".to_string(),
            service_endpoint: format!("/ipns/{}", self.ipns_keypair.ipns_name),
        };
        
        // 创建libp2p节点服务（包含NodeInfo的JSON）
        let node_info_json = serde_json::to_string(node_info)?;
        let libp2p_service = Service {
            id: "#libp2p-node".to_string(),
            service_type: "LibP2PNode".to_string(),
            service_endpoint: node_info_json,
        };
        
        // 构建完整的DID文档
        Ok(DIDDocument {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: self.ipns_keypair.did.clone(),
            verification_method: vec![
                ipns_verification_method,
                libp2p_verification_method,
            ],
            authentication: vec![
                format!("{}#ipns-key", self.ipns_keypair.did),
                format!("{}#libp2p-key", self.ipns_keypair.did),
            ],
            service: Some(vec![
                ipns_service,
                libp2p_service,
            ]),
            ipfs_metadata: None,  // 稍后会添加
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_manager::KeyPair;
    
    #[test]
    fn test_startup_manager_creation() {
        let ipns_keypair = KeyPair::generate().unwrap();
        let libp2p_identity = LibP2PIdentity::generate().unwrap();
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let ipns_publisher = IpnsPublisher::new(true, false, None, 365);
        let config = StartupConfig::default();
        
        let manager = StartupManager::new(
            ipns_keypair,
            libp2p_identity,
            ipfs_client,
            ipns_publisher,
            config,
        );
        
        assert!(manager.config.always_update);
    }
}
