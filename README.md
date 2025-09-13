# ANP Rust SDK

[![Crates.io](https://img.shields.io/crates/v/anp-rs-sdk.svg)](https://crates.io/crates/anp-rs-sdk)
[![Documentation](https://docs.rs/anp-rs-sdk/badge.svg)](https://docs.rs/anp-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**ANP Rust SDK** 是智能体网络协议（Agent Network Protocol）的 Rust 实现，提供完整的自动配置工具包，包括 HTTP 服务器自动配置、DID 自动生成、智能体描述等功能。

## 🚀 特性

- **HTTP 服务器自动配置**：自动端口分配、路由管理、CORS 支持
- **DID 自动生成**：支持 Ed25519、secp256k1、X25519 等多种加密算法
- **智能体描述**：自动生成符合 ANP 标准的智能体描述文档
- **异步支持**：基于 Tokio 的高性能异步运行时
- **类型安全**：完整的 Rust 类型系统支持
- **易于使用**：简单的 API 设计，快速上手

## 📦 安装

在你的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
anp-rs-sdk = "1.0.0"
```

## 🎯 快速开始

### 基础 HTTP 配置

```rust
use anp_rs_sdk::{ANPSDK, AutoConfigOptions};

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
# 基础 HTTP 配置
cargo run --example basic_http_config

# DID 配置
cargo run --example basic_did_config

# 完整 ANP 智能体
cargo run --example full_anp_agent

# 自定义配置
cargo run --example custom_config
```

## 🔧 API 文档

完整的 API 文档可以在 [docs.rs](https://docs.rs/anp-rs-sdk) 上找到。

## 🤝 贡献

我们欢迎社区贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解如何参与开发。

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](LICENSE) 文件。

## 🔗 相关链接

- [ANP 官方网站](https://agent-network-protocol.com)
- [ANP 技术白皮书](https://github.com/logos-42/AgentNetworkProtocol)
- [W3C WebAgents 社区组](https://www.w3.org/community/webagents/)

## 🆕 更新日志

### v1.0.0
- 初始版本发布
- 支持 HTTP 服务器自动配置
- 支持 DID 自动生成
- 支持智能体描述生成
- 提供完整的示例和文档