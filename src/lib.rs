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

// 内置IPFS节点管理器
pub mod ipfs_node_manager;

// DID构建器（简化版）
pub mod did_builder;

// libp2p身份
pub mod libp2p_identity;
pub mod libp2p_node;
pub mod p2p_communicator;

// 签名PeerID（隐私保护）
pub mod encrypted_peer_id;

// ZKP模块
pub mod zkp_circuit;
pub mod zkp_prover;
pub mod zkp_setup;

// 统一身份管理
pub mod identity_manager;

// Nonce管理器（防重放攻击）
pub mod nonce_manager;

// DID文档缓存
pub mod did_cache;

// IPFS Pubsub认证通讯
pub mod pubsub_authenticator;

// Noir ZKP集成（新版本）
pub mod noir_zkp;
pub mod noir_verifier;

// 统一ZKP接口（解决功能错位问题）
pub mod unified_zkp;

// 智能体验证闭环
pub mod agent_verification;

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
pub use key_manager::{
    KeyPair, KeyManager, KeyBackup
};

// IPFS客户端
pub use ipfs_client::{
    IpfsClient, IpfsUploadResult
};

// 内置IPFS节点管理器
pub use ipfs_node_manager::{
    IpfsNodeManager,
    IpfsNodeConfig,
    IpfsNodeStatus,
    IpfsNodeInfo,
};

// DID构建器
pub use did_builder::{
    DIDBuilder, DIDPublishResult, 
    DIDDocument, 
    VerificationMethod,
    Service,
    get_did_document_from_cid,
    verify_did_document_integrity,
};

// libp2p模块
pub use libp2p_identity::{
    LibP2PIdentity, LibP2PIdentityManager
};

pub use libp2p_node::{
    LibP2PNode, NodeInfo
};

pub use p2p_communicator::{
    P2PCommunicator, DIAPMessage, DIAPResponse
};

// 签名PeerID（隐私保护）
pub use encrypted_peer_id::{
    EncryptedPeerID,
    encrypt_peer_id,
    decrypt_peer_id_with_secret,
    verify_peer_id_signature,
    verify_encrypted_peer_id_ownership,
};

// ZKP模块
pub use zkp_circuit::{
    DIDBindingCircuit,
    CircuitParams,
};

pub use zkp_prover::{
    ZKPProver,
    ZKPVerifier,
    ProofResult,
    generate_trusted_setup,
};

pub use zkp_setup::{
    ZKPSetup,
};

// Noir ZKP集成
pub use noir_zkp::{
    NoirZKPManager,
    NoirAgent,
    NoirProofResult,
    PerformanceMetrics,
    NoirProverInputs,
};

// Noir验证器
pub use noir_verifier::{
    NoirVerifier,
    NoirVerificationResult,
    ImprovedNoirZKPManager,
};

// 统一ZKP接口
pub use unified_zkp::{
    UnifiedZKPManager,
    UnifiedZKPInputs,
    UnifiedZKPOutput,
    UnifiedVerificationResult,
    ZKPScheme,
    ZKPPerformanceTester,
    ZKPPerformanceComparison,
    ZKPSchemeResults,
};

// 智能体验证闭环
pub use agent_verification::{
    AgentVerificationManager,
    AgentVerificationRequest,
    AgentVerificationResponse,
    AgentVerificationStatus,
    CacheStats,
};

// 智能体认证管理器
pub use agent_auth::{
    AgentAuthManager,
    AuthResult,
    BatchAuthResult,
};

// ZKP密钥生成器
pub use key_generator::{
    generate_simple_zkp_keys,
    ensure_zkp_keys_exist,
    generate_noir_keys,
};

// 身份管理
pub use identity_manager::{
    IdentityManager,
    AgentInfo,
    ServiceInfo,
    IdentityRegistration,
    IdentityVerification,
};

// 配置管理
pub use config_manager::{
    DIAPConfig,
    AgentConfig,
    IpfsConfig,
    IpnsConfig,
    CacheConfig,
    LoggingConfig,
};

// Nonce管理器
pub use nonce_manager::{
    NonceManager,
    NonceRecord,
};

// DID文档缓存
pub use did_cache::{
    DIDCache,
    CacheEntry,
    CacheStats as DIDCacheStats,
};

// Pubsub认证器
pub use pubsub_authenticator::{
    PubsubAuthenticator,
    AuthenticatedMessage,
    MessageVerification,
    TopicPolicy,
    TopicConfig,
};

// Iroh节点
pub use iroh_node::{
    IrohNode,
    IrohConfig,
};

// ============ 常用类型重导出 ============
pub use serde::{Deserialize, Serialize};
pub use anyhow::Result;

// ============ 版本信息 ============
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = "DIAP Rust SDK - ZKP版本：使用零知识证明验证DID-CID绑定 + IPFS Pubsub认证通讯";
