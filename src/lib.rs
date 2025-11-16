/**
 * DIAP Rust SDK - ZKP版本
 * Decentralized Intelligent Agent Protocol
 * 使用零知识证明验证DID-CID绑定，无需IPNS
 */
// ============ 核心模块 ============

// 密钥管理
pub mod key_manager;

// IPFS客户端
pub mod ipfs_client;

// 内置IPFS节点管理器（Kubo 特性暂未启用）
// 注：移除未定义的 feature 开关以消除编译警告
// pub mod ipfs_node_manager;

// Kubo自动安装器
pub mod kubo_installer;

// DID构建器（简化版）
pub mod did_builder;

// libp2p身份
pub mod libp2p_identity;
pub mod libp2p_node;

// 签名PeerID（隐私保护）
pub mod encrypted_peer_id;
// Iroh ID 加密
pub mod encrypted_iroh_id;

// ZKP模块 (基于Noir)

// 统一身份管理
pub mod identity_manager;

// Nonce管理器（防重放攻击）
pub mod nonce_manager;

// DID文档缓存
pub mod did_cache;

// IPFS Pubsub认证通讯
pub mod pubsub_authenticator;

// Noir ZKP集成（新版本）
pub mod noir_verifier;
pub mod noir_zkp;

// 智能体验证闭环
pub mod agent_verification;

// IPFS双向验证系统
pub mod ipfs_bidirectional_verification;

// 智能体认证管理器（统一API）
pub mod agent_auth;

// ZKP密钥生成器
pub mod key_generator;

// Iroh节点（预留）
pub mod iroh_node;

// 配置管理（保留）
pub mod config_manager;

// ============ 公共导出 ============

// 密钥管理
pub use key_manager::{KeyBackup, KeyManager, KeyPair};

// IPFS客户端
pub use ipfs_client::{IpfsClient, IpfsUploadResult, IpnsPublishResult};

// 内置IPFS节点管理器导出（Kubo 特性暂未启用）
// pub use ipfs_node_manager::{
//     IpfsNodeManager,
//     IpfsNodeConfig,
//     IpfsNodeStatus,
//     IpfsNodeInfo,
// };

// Kubo自动安装器
pub use kubo_installer::KuboInstaller;

// DID构建器
pub use did_builder::{
    get_did_document_from_cid, verify_did_document_integrity, DIDBuilder, DIDDocument,
    DIDPublishResult, Service, VerificationMethod,
};

// libp2p模块
pub use libp2p_identity::{LibP2PIdentity, LibP2PIdentityManager};

pub use libp2p_node::{LibP2PNode, NodeInfo};

// Iroh P2P通信器
pub mod iroh_communicator;

// 签名PeerID（隐私保护）
pub use encrypted_peer_id::{
    decrypt_peer_id_with_secret, encrypt_peer_id, verify_encrypted_peer_id_ownership,
    verify_peer_id_signature, EncryptedPeerID,
};

// ZKP模块 (基于Noir)

// 嵌入Noir电路模块
#[cfg(feature = "embedded-noir")]
pub mod noir_embedded;

// 通用Noir管理器
pub mod noir_universal;

// Noir ZKP集成
pub use noir_zkp::{
    NoirAgent, NoirProofResult, NoirProverInputs, NoirZKPManager, PerformanceMetrics,
};

// Noir验证器
pub use noir_verifier::{ImprovedNoirZKPManager, NoirVerificationResult, NoirVerifier};

// 导出通用管理器
pub use noir_universal::{BackendInfo, NoirBackend, PerformanceStats, UniversalNoirManager};

// 导出嵌入模块（如果启用）
#[cfg(feature = "embedded-noir")]
pub use noir_embedded::{
    CacheStats as EmbeddedCacheStats, CircuitMetadata, EmbeddedCircuit, EmbeddedNoirZKPManager,
};

// 智能体验证闭环
pub use agent_verification::{
    AgentVerificationManager, AgentVerificationRequest, AgentVerificationResponse,
    AgentVerificationStatus, CacheStats,
};

// IPFS双向验证系统
pub use ipfs_bidirectional_verification::{
    AgentSession, BidirectionalVerificationResult, IpfsBidirectionalVerificationManager, ProofData,
    SessionStatus, VerificationChallenge, VerificationResult, VerificationStatus,
};

// 智能体认证管理器
pub use agent_auth::{AgentAuthManager, AuthResult, BatchAuthResult};

// ZKP密钥生成器
pub use key_generator::{ensure_zkp_keys_exist, generate_noir_keys, generate_simple_zkp_keys};

// 身份管理
pub use identity_manager::{
    AgentInfo, IdentityManager, IdentityRegistration, IdentityVerification, ServiceInfo,
};

// 配置管理
pub use config_manager::{
    AgentConfig, CacheConfig, DIAPConfig, IpfsConfig, IpnsConfig, LoggingConfig,
};

// Nonce管理器
pub use nonce_manager::{NonceManager, NonceRecord};

// DID文档缓存
pub use did_cache::{CacheEntry, CacheStats as DIDCacheStats, DIDCache};

// Pubsub认证器
pub use pubsub_authenticator::{
    AuthenticatedMessage, MessageVerification, PubSubMessageType, PubsubAuthRequestPayload,
    PubsubAuthResponsePayload, PubsubAuthenticator, TopicConfig, TopicPolicy,
};

// Iroh节点
pub use iroh_node::{IrohConfig, IrohNode};

// Iroh P2P通信器
pub use iroh_communicator::{
    IrohCommunicator, IrohConfig as IrohCommConfig, IrohConnection, IrohMessage, IrohMessageType,
};

// ============ 常用类型重导出 ============
pub use anyhow::Result;
pub use serde::{Deserialize, Serialize};

// ============ 版本信息 ============
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str =
    "DIAP Rust SDK - Noir ZKP版本：基于Noir零知识证明的去中心化智能体身份验证系统";
