# DANP Rust SDK

[![Crates.io](https://img.shields.io/crates/v/anp-rs-sdk.svg)](https://crates.io/crates/DIAP_Rust_SDK)
[![Documentation](https://docs.rs/DIAP_Rust_SDK/badge.svg)](https://docs.rs/DIAP_Rust_SDK)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP Rust SDK** 是智能体网络协议（Decentralized Intelligent Agent Protocol）的 Rust 实现，提供完整的自动配置工具包，包括 HTTP 服务器自动配置、DID 自动生成、智能体描述等功能。

## 🚀 特性

- **HTTP 服务器自动配置**：自动端口分配、路由管理、CORS 支持
- **DID 自动生成**：支持 Ed25519、secp256k1、X25519 等多种加密算法
- **多 DID 格式支持**：同时支持 `did:wba` 和 `did:web` 格式
- **真实路由输出**：DID 文档、AD 文档通过 HTTP 端点真实可访问
- **IPFS 注册表**：支持将智能体信息发布到 IPFS 网络，实现去中心化发现
- **智能体描述**：自动生成符合 ANP 标准的智能体描述文档
- **异步支持**：基于 Tokio 的高性能异步运行时
- **类型安全**：完整的 Rust 类型系统支持
- **易于使用**：简单的 API 设计，快速上手

## 📦 安装

在你的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
diap-rs-sdk = "0.1.3"
```

## 🎯 快速开始

### 基础 HTTP 配置

```rust
use DIAP::{DIAPSDK, AutoConfigOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_port: Some(true),
        ..Default::default()
    };

    let mut sdk = ANPSDK::new(options);
    let config = sdk.start().await?;
    
    println!("HTTP服务器启动在: {}", config.endpoint);
    
    // 你的应用逻辑...
    
    sdk.stop().await?;
    Ok(())
}
```

### DID 配置

```rust
use anp_rs_sdk::did_auto_config::{DIDAutoConfig, DIDAutoConfigOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = DIDAutoConfigOptions {
        agent_name: Some("My ANP Agent".to_string()),
        ..Default::default()
    };

    let mut did_config = DIDAutoConfig::new(options);
    let config = did_config.generate_did().await?;
    
    println!("DID: {}", config.did);
    println!("DID文档: {}", serde_json::to_string_pretty(&config.did_document)?);
    
    Ok(())
}
```

### 完整 ANP 智能体

```rust
use anp_rs_sdk::{ANPSDK, AutoConfigOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = AutoConfigOptions {
        auto_start: Some(true),
        auto_did: Some(true),
        agent_name: Some("My ANP Agent".to_string()),
        ..Default::default()
    };

    let mut sdk = ANPSDK::new(options);
    let config = sdk.start().await?;
    
    println!("🎉 ANP智能体启动成功！");
    println!("- HTTP端点: {}", config.endpoint);
    println!("- DID: {}", config.did);
    println!("- DID文档: {}/.well-known/did.json", config.endpoint);
    println!("- 智能体描述: {}/agents/auto-agent/ad.json", config.endpoint);
    
    // 等待一段时间
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    sdk.stop().await?;
    Ok(())
}
```

## 📚 示例

SDK 提供了多个示例来帮助你快速上手：

```bash
# 基础示例（包含 did:web 支持）
cargo run --example basic_agent_with_did_web

# 完整示例（包含 IPFS 注册）
cargo run --example complete_agent_with_ipfs

# IPFS 注册表演示
cargo run --example ipfs_registry_demo

# 传统示例
cargo run --example basic_http_config
cargo run --example basic_did_config
cargo run --example full_anp_agent
cargo run --example custom_config
```

### 新功能亮点

#### 1. 双 DID 格式支持
```rust
let config = sdk.start().await?;
println!("DID (wba): {}", config.did);
println!("DID (web): {}", config.did_web.unwrap());
```

#### 2. 真实的 HTTP 端点
- `GET /.well-known/did.json` - 返回真实的 DID 文档
- `GET /agents/{id}/ad.json` - 返回智能体描述文档
- `POST /anp/api` - ANP 协议通信端点

#### 3. IPFS 注册表
```rust
let options = AutoConfigOptions {
    auto_ipfs_register: Some(true),
    ipfs_config: Some(IpfsRegistryConfig {
        api_url: "http://127.0.0.1:5001".to_string(),
        gateway_url: "https://ipfs.io".to_string(),
        pin: true,
    }),
    ..Default::default()
};
```


#### 1. **真实的 HTTP 路由输出**
- ✅ 修复了 DID 文档路由，`GET /.well-known/did.json` 现在返回真实的 DID 文档
- ✅ 修复了 AD 文档路由，`GET /agents/{id}/ad.json` 现在返回真实的智能体描述文档
- ✅ 实现了 ANP API 端点，`POST /anp/api` 可以接收和处理 ANP 协议消息
- ✅ 添加了适当的错误处理和 404 响应

#### 2. **双 DID 格式支持**
- ✅ 同时支持 `did:wba` 和 `did:web` 格式
- ✅ `KeyPairResult` 现在包含 `did_web` 字段
- ✅ `DIDConfig` 和 `AgentConfig` 都包含 `did_web` 字段
- ✅ 自动生成符合 W3C 标准的 `did:web` 标识符

**示例**:
```rust
let config = sdk.start().await?;
println!("DID (wba): {}", config.did);
// 输出: did:wba:127.0.0.1:3000:auto-agent

println!("DID (web): {}", config.did_web.unwrap());
// 输出: did:web:127.0.0.1%3A3000:auto-agent
```

#### 3. **IPFS 注册表集成**
- ✅ 新增 `ipfs_registry` 模块，提供完整的 IPFS 注册表功能
- ✅ 支持将智能体信息发布到 IPFS 网络
- ✅ 支持从 IPFS 查询智能体信息
- ✅ 支持发布和查询注册表索引（多个智能体的列表）
- ✅ 支持按能力、接口等条件搜索智能体
- ✅ 可配置的 IPFS API 和网关地址
- ✅ 支持内容固定（pin）到本地节点

**使用方式**:
```rust
// 自动注册到 IPFS
let options = AutoConfigOptions {
    auto_ipfs_register: Some(true),
    ipfs_config: Some(IpfsRegistryConfig {
        api_url: "http://127.0.0.1:5001".to_string(),
        gateway_url: "https://ipfs.io".to_string(),
        pin: true,
    }),
    ..Default::default()
};

let config = sdk.start().await?;
if let Some(cid) = config.ipfs_cid {
    println!("IPFS CID: {}", cid);
    println!("访问: https://ipfs.io/ipfs/{}", cid);
}
```

**手动使用**:
```rust
use anp_rs_sdk::{IpfsRegistry, IpfsRegistryConfig, AgentRegistryEntry};

let registry = IpfsRegistry::new(IpfsRegistryConfig::default());

// 发布智能体
let cid = registry.publish_agent(entry).await?;

// 查询智能体
let agent = registry.query_agent(&cid).await?;

// 搜索智能体
let results = registry.search_agents(&index_cid, filter).await?;
```

### 📚 新增示例

1. **basic_agent_with_did_web.rs**
   - 展示双 DID 格式支持
   - 演示真实的 HTTP 端点
   - 自动测试所有端点
   - 适合快速入门

2. **complete_agent_with_ipfs.rs**
   - 完整功能演示
   - 包含 IPFS 注册
   - 展示端到端工作流
   - 适合了解全部功能

3. **ipfs_registry_demo.rs**
   - 专注于 IPFS 功能
   - 演示发布、查询、搜索
   - 包含故障排除提示
   - 适合 IPFS 集成开发

### 🔧 API 变更

#### 新增类型

```rust
// IPFS 注册表配置
pub struct IpfsRegistryConfig {
    pub api_url: String,
    pub gateway_url: String,
    pub pin: bool,
}

// 智能体注册信息
pub struct AgentRegistryEntry {
    pub did: String,
    pub did_web: Option<String>,
    pub name: String,
    pub endpoint: String,
    pub did_document_url: String,
    pub ad_url: String,
    pub capabilities: Vec<String>,
    pub interfaces: Vec<String>,
    pub registered_at: String,
    pub updated_at: String,
}

// IPFS 注册表
pub struct IpfsRegistry { /* ... */ }

// 搜索过滤器
pub struct AgentSearchFilter {
    pub did: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub interfaces: Option<Vec<String>>,
}
```

#### 修改的类型

```rust
// AutoConfigOptions 新增字段
pub struct AutoConfigOptions {
    // ... 原有字段 ...
    pub auto_ipfs_register: Option<bool>,  // 新增
    pub ipfs_config: Option<IpfsRegistryConfig>,  // 新增
}

// AgentConfig 新增字段
pub struct AgentConfig {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
    pub ipfs_cid: Option<String>,  // 新增
}

// KeyPairResult 新增字段
pub struct KeyPairResult {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
}

// DIDConfig 新增字段
pub struct DIDConfig {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
}
```

#### 新增方法

```rust
// HTTPAutoConfig
impl HTTPAutoConfig {
    pub async fn set_did_document(&self, doc: Value);
    pub async fn set_ad_document(&self, doc: Value);
}

// IpfsRegistry
impl IpfsRegistry {
    pub fn new(config: IpfsRegistryConfig) -> Self;
    pub async fn publish_agent(&self, entry: AgentRegistryEntry) -> Result<String>;
    pub async fn query_agent(&self, cid: &str) -> Result<AgentRegistryEntry>;
    pub async fn publish_registry_index(&self, entries: Vec<AgentRegistryEntry>) -> Result<String>;
    pub async fn query_registry_index(&self, cid: &str) -> Result<RegistryIndex>;
    pub async fn search_agents(&self, index_cid: &str, filter: AgentSearchFilter) -> Result<Vec<AgentRegistryEntry>>;
}
```

### 📦 依赖更新

- `reqwest`: 添加 `multipart` feature
- `multipart`: 新增依赖 0.18

### 🐛 修复

- 修复了 HTTP 路由只返回占位符的问题
- 修复了 DID 文档和 AD 文档无法访问的问题
- 改进了错误处理和用户反馈

### 📖 文档改进

- 更新了 README.md，添加新功能说明
- 添加了详细的使用示例
- 添加了 IPFS 集成指南
- 改进了 API 文档注释

### ⚠️ 注意事项

1. **IPFS 功能**：
   - 需要本地运行 IPFS 节点（`ipfs daemon`）
   - 默认 API 端口: 5001
   - 默认网关端口: 8080
   - 可以使用公共 IPFS 网关查看内容

2. **DID Web 格式**：
   - `did:web` 格式中的端口号使用 `%3A` 编码
   - 示例: `did:web:example.com%3A3000:agent`
   - 解析时需要将 `:` 替换回端口号

3. **向后兼容**：
   - 所有新增字段都是 `Option<T>` 类型
   - 现有代码无需修改即可升级
   - IPFS 功能默认关闭

### 🚀 使用建议

#### 快速开始（不使用 IPFS）
```bash
cargo run --example basic_agent_with_did_web
```

#### 完整功能（包含 IPFS）
```bash
# 1. 启动 IPFS 节点
ipfs daemon

# 2. 运行示例
cargo run --example complete_agent_with_ipfs
```

#### 集成到现有项目
```rust
use anp_rs_sdk::{ANPSDK, AutoConfigOptions};

let options = AutoConfigOptions {
    auto_ipfs_register: Some(false),  // 暂不使用 IPFS
    ..Default::default()
};

let mut sdk = ANPSDK::new(options);
let config = sdk.start().await?;

// 访问 DID 文档
// GET http://127.0.0.1:{port}/.well-known/did.json

// 访问 AD 文档
// GET http://127.0.0.1:{port}/agents/auto-agent/ad.json
```

---

## [0.1.1] - 2025-10-06

### 🐛 修复
- 修复了编译警告
- 清理了未使用的代码

---


## 🔧 API 文档

完整的 API 文档可以在 [docs.rs](https://docs.rs/anp-rs-sdk) 上找到。

## 🤝 贡献

我们欢迎社区贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与开发。

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](LICENSE) 文件。

## 🔗 相关链接

- [ANP 官方网站](https://github.com/agent-network-protocol/AgentNetworkProtocol)
- [ANP 技术白皮书](https://github.com/agent-network-protocol/AgentNetworkProtocol)
- [W3C WebAgents 社区组](https://www.w3.org/community/webagents/)

## 🆕 更新日志

## [0.1.2] - 2025-10-06

### 🎉 初始版本
- HTTP 服务器自动配置
- DID 自动生成（Ed25519、secp256k1）
- 智能体描述文档生成
- 基础示例和文档

