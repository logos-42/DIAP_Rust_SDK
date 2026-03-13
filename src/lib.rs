/**
 * DIAP Rust SDK - ZKP 版本
 * Decentralized Intelligent Agent Protocol
 * 使用零知识证明验证 DID-CID 绑定，无需 IPNS
 */
// 密钥管理
pub mod key_manager;

// IPFS 客户端
pub mod ipfs_client;

// IPNS 管理器
pub mod ipns_manager;

// 内置 IPFS 节点管理器（Kubo 特性暂未启用）
pub mod kubo_installer;

// DID 构建器（简化版）
pub mod did_builder;

// 身份管理器
pub mod identity_manager;

// Agent 认证
pub mod agent_auth;

// PubSub 认证
pub mod pubsub_authenticator;

// Noir ZKP
pub mod noir_zkp;

// Noir 嵌入式
pub mod noir_embedded;

// ZKP 密钥生成器
pub mod key_generator;

// Iroh P2P 通信器
pub mod iroh_communicator;

// 加密 Peer ID
pub mod encrypted_peer_id;

// 加密 Iroh ID
pub mod encrypted_iroh_id;

// DID 缓存
pub mod did_cache;

// Nonce 管理器
pub mod nonce_manager;

// Noir 验证器
pub mod noir_verifier;

// Agent 验证
pub mod agent_verification;

// 配置管理器
pub mod config_manager;

// Iroh 节点
pub mod iroh_node;

// Iroh API 研究（仅用于参考）
// pub mod iroh_api_research;

// IPFS 双向验证
pub mod ipfs_bidirectional_verification;

// IPFS 节点管理器
pub mod ipfs_node_manager;

// 重新导出主要类型
pub use key_manager::{KeyPair, KeyManager};
pub use identity_manager::{IdentityManager, AgentInfo};
pub use agent_auth::AgentAuthManager;
pub use did_builder::DIDDocument;
pub use identity_manager::{IdentityRegistration, ServiceInfo};
pub use ipfs_client::IpfsClient;
pub use ipns_manager::{IpnsManager, IpnsPublishResult, KeyInfo};
pub use agent_verification::{
    AgentVerificationManager, AgentVerificationRequest, AgentVerificationStatus,
};
pub use did_builder::{VerificationMethod, Service};
pub use did_cache::CacheStats;

/// SDK 版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
