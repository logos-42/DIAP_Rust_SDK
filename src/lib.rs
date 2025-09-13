/**
 * ANP Rust SDK 主入口文件
 * 整合所有ANP自动配置功能
 */

// 导出核心模块
pub mod anp_key_generator;
pub mod http_auto_config;
pub mod did_auto_config;
pub mod auto_config;

// 导出核心结构体和枚举
pub use anp_key_generator::{
    ANPKeyGenerator, KeyType, KeyPairResult, DIDDocument, 
    VerificationMethod, Service, SignatureData
};

pub use http_auto_config::{
    HTTPAutoConfig, HTTPAutoConfigOptions, RouteConfig, HTTPConfig
};

pub use did_auto_config::{
    DIDAutoConfig, DIDAutoConfigOptions, ServiceEndpoint, DIDConfig, AgentInterface
};

pub use auto_config::{
    ANPSDK, AutoConfigAgent, ANPClient, AutoConfigOptions, 
    AgentConfig, ANPRequest, ANPResponse
};

// 重新导出常用类型
pub use serde::{Deserialize, Serialize};
pub use anyhow::Result;

// 默认导出主SDK结构体
pub use ANPSDK as default;
