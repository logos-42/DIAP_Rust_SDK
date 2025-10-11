/**
 * DIAP Rust SDK 主入口文件
 * Decentralized Intelligent Agent Protocol
 * 整合所有去中心化智能体协议自动配置功能
 */

// 导出核心模块
pub mod diap_key_generator;
pub mod http_auto_config;
pub mod did_auto_config;
pub mod auto_config;
pub mod ipfs_registry;

// 新增模块 - IPFS/IPNS集成
pub mod config_manager;
pub mod key_manager;
pub mod ipfs_client;
pub mod ipns_publisher;
pub mod did_builder;
pub mod did_resolver;
pub mod batch_uploader;

// libp2p集成
pub mod libp2p_identity;
pub mod libp2p_node;
pub mod startup_manager;
pub mod p2p_communicator;

// 统一身份管理模块
pub mod identity_manager;

// 导出核心结构体和枚举
pub use diap_key_generator::{
    DIAPKeyGenerator, KeyType, KeyPairResult, DIDDocument, 
    VerificationMethod, Service, SignatureData
};

pub use http_auto_config::{
    HTTPAutoConfig, HTTPAutoConfigOptions, RouteConfig, HTTPConfig
};

pub use did_auto_config::{
    DIDAutoConfig, DIDAutoConfigOptions, ServiceEndpoint, DIDConfig, AgentInterface
};

pub use auto_config::{
    DIAPSDK, AutoConfigAgent, DIAPClient, AutoConfigOptions, 
    AgentConfig, DIAPRequest, DIAPResponse as AutoConfigDIAPResponse
};

pub use ipfs_registry::{
    IpfsRegistry, IpfsRegistryConfig, AgentRegistryEntry,
    RegistryIndex, AgentSearchFilter
};

// 新增模块导出
pub use config_manager::{
    DIAPConfig, AgentConfig as ConfigAgentConfig, IpfsConfig, 
    IpnsConfig, CacheConfig, LoggingConfig
};

pub use key_manager::{
    KeyPair, KeyManager, KeyBackup
};

pub use ipfs_client::{
    IpfsClient, IpfsUploadResult
};

pub use ipns_publisher::{
    IpnsPublisher, IpnsPublishResult, IpnsRecord
};

pub use did_builder::{
    DIDBuilder, DIDPublishResult, 
    DIDDocument as NewDIDDocument, 
    VerificationMethod as NewVerificationMethod,
    Service as NewService,
    IpfsMetadata,
    verify_double_layer
};

pub use did_resolver::{
    DIDResolver, ResolveResult
};

pub use batch_uploader::{
    BatchUploader, BatchUploadResult, BatchItemResult,
    AutoUpdateManager
};

// libp2p模块导出
pub use libp2p_identity::{
    LibP2PIdentity, LibP2PIdentityManager
};

pub use libp2p_node::{
    LibP2PNode, NodeInfo
};

pub use startup_manager::{
    StartupManager, StartupConfig
};

pub use p2p_communicator::{
    P2PCommunicator, DIAPMessage, DIAPResponse
};

// 统一身份管理模块导出
pub use identity_manager::{
    IdentityManager, AgentInfo, ServiceInfo,
    IdentityRegistration, IdentityVerification
};

// 重新导出常用类型
pub use serde::{Deserialize, Serialize};
pub use anyhow::Result;

// 默认导出主SDK结构体
pub use DIAPSDK as default;
