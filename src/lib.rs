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

// DID构建器（简化版）
pub mod did_builder;

// libp2p身份
pub mod libp2p_identity;
pub mod libp2p_node;
pub mod p2p_communicator;

// 加密PeerID
pub mod encrypted_peer_id;

// ZKP模块
pub mod zkp_circuit;
pub mod zkp_prover;

// 统一身份管理
pub mod identity_manager;

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

// 加密PeerID
pub use encrypted_peer_id::{
    EncryptedPeerID,
    encrypt_peer_id,
    decrypt_peer_id_with_secret,
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

// ============ 常用类型重导出 ============
pub use serde::{Deserialize, Serialize};
pub use anyhow::Result;

// ============ 版本信息 ============
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = "DIAP Rust SDK - ZKP版本：使用零知识证明验证DID-CID绑定";
