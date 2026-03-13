# DIAP Rust SDK

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

基于零知识证明的去中心化智能体身份协议 Rust SDK，支持跨平台零依赖部署。

## 📦 版本选择

### 🔷 Kubo 分支（完整功能版）- 当前分支
**适用于**：云服务器、完整节点部署

- ✅ 使用 Kubo（go-ipfs）作为完整 IPFS 节点
- ✅ 自动启动和管理本地 IPFS 守护进程
- ✅ 支持完整的 IPFS DHT 网络
- ✅ 独立的 IPNS 管理模块
- ✅ 适合生产环境部署

### 🔷 Helia 分支（轻量版）
**适用于**：边缘计算、资源受限环境

- ✅ 轻量级 HTTP 客户端，无需本地 IPFS 守护进程
- ✅ 仅使用 HTTP API 连接到远程 IPFS 节点
- ✅ 适合边缘服务器、IoT 设备

## 📢 最新版本：0.2.15

**重要更新**：
- 🆕 独立的 `IpnsManager` 模块，集中管理 IPNS 功能
- 🚀 并行发布到多个节点，加速 DHT 传播（3 倍速度提升）
- 📡 DHT 广播和预传播功能
- 🔧 修复依赖问题（bincode 3.0 恶作剧版本）
- 📝 改进的序列化处理

## 快速开始

### 安装

```toml
[dependencies]
diap-rs-sdk = "0.2.15"
tokio = { version = "1.0", features = ["full"] }
env_logger = "0.11"
anyhow = "1.0"
```

### 基本使用

#### 1. 创建智能体和 DID

```rust
use diap_rs_sdk::{AgentAuthManager, KeyPair};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // 创建智能体
    let auth_manager = AgentAuthManager::new().await?;
    let (agent_info, keypair, peer_id) = auth_manager.create_agent("MyAgent", None)?;

    println!("智能体创建成功：{}", agent_info.name);
    println!("DID: {}", keypair.did);
    println!("PeerID: {}", peer_id);

    Ok(())
}
```

#### 2. 使用 IPNS 管理器发布 DID 文档

```rust
use diap_rs_sdk::{IpnsManager, IpfsClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建 IPNS 管理器
    let ipns = IpnsManager::new_with_api(
        "http://localhost:5001".to_string(),
        "http://localhost:8080".to_string(),
    );

    // 假设已经上传到 IPFS 获取 CID
    let cid = "QmYourCIDHere";
    
    // 发布到 IPNS（DHT 直接传播）
    let result = ipns.publish_direct(&cid, "my-did-key", "8760h", "1h").await?;
    println!("✅ IPNS 发布成功：/ipns/{}", result.name);
    println!("🌐 全球访问：https://ipfs.io/ipns/{}", result.name);

    Ok(())
}
```

#### 3. 并行发布到多个节点（加速传播）

```rust
use diap_rs_sdk::IpnsManager;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ipns = IpnsManager::new_local();
    let cid = "QmYourCIDHere";

    // 并行发布到 3 个节点
    let nodes = vec![
        "http://node1:5001",
        "http://node2:5001",
        "http://node3:5001",
    ];

    let results = ipns.publish_to_multiple_nodes(
        &cid,
        "my-did-key",
        &nodes,
        "8760h",
        "1h"
    ).await?;

    println!("✅ 成功发布到 {}/{} 个节点", results.len(), nodes.len());

    Ok(())
}
```

## 🆕 IPNS 管理器 API

### 核心功能

| 方法 | 说明 | 示例 |
|------|------|------|
| `publish()` | 普通模式发布 | `ipns.publish(&cid, "key", "8760h", "1h")` |
| `publish_direct()` | DHT 直接发布 | `ipns.publish_direct(&cid, "key", "8760h", "1h")` |
| `publish_to_multiple_nodes()` | 并行发布到多个节点 | `ipns.publish_to_multiple_nodes(&cid, "key", &nodes, "8760h", "1h")` |
| `publish_and_pin()` | 发布 + Pin 组合 | `ipns.publish_and_pin(&cid, "key", "8760h", "1h", &pin_nodes)` |
| `batch_publish_ipns()` | 批量发布到多个 key | `ipns.batch_publish_ipns(&cid, vec!["key1", "key2"], "8760h")` |
| `resolve()` | 解析 IPNS 名称 | `ipns.resolve(&ipns_name)` |
| `broadcast_to_dht()` | 广播到 DHT | `ipns.broadcast_to_dht(&ipns_name)` |
| `prepropagate_cid()` | 预传播 CID | `ipns.prepropagate_cid(&cid, &bootstrap_nodes)` |

### Key 管理

```rust
use diap_rs_sdk::IpnsManager;

let ipns = IpnsManager::new_local();

// 确保 key 存在
ipns.ensure_key_exists("my-key").await?;

// 列出所有 keys
let keys = ipns.list_keys().await?;
for key in keys {
    println!("Key: {} -> {}", key.name, key.id);
}

// 删除 key
ipns.remove_key("my-key").await?;
```

### 完整发布流程示例

```rust
use diap_rs_sdk::{IpnsManager, IpfsClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let ipns = IpnsManager::new_local();
    let ipfs = IpfsClient::new_local();

    // 1. 上传 DID 文档到 IPFS
    let did_doc = r#"{"@context": "https://w3id.org/did/v1", ...}"#;
    let upload_result = ipfs.upload(did_doc, "did.json").await?;
    let cid = upload_result.cid;

    // 2. 预传播 CID 到引导节点（加速首次访问）
    let bootstrap_nodes = vec!["http://bootstrap1:5001", "http://bootstrap2:5001"];
    ipns.prepropagate_cid(&cid, bootstrap_nodes).await?;

    // 3. 并行发布到多个节点
    let nodes = vec![
        "http://node1:5001",
        "http://node2:5001",
        "http://node3:5001",
    ];
    let results = ipns.publish_to_multiple_nodes(
        &cid, "my-did-key", &nodes, "8760h", "1h"
    ).await?;

    // 4. Pin 到额外节点确保可用性
    let pin_nodes = vec!["http://storage1:5001", "http://storage2:5001"];
    ipns.publish_and_pin(&cid, "my-did-key", "8760h", "1h", pin_nodes).await?;

    // 5. 广播到 DHT
    if let Some(result) = results.first() {
        ipns.broadcast_to_dht(&result.name).await?;
        println!("✅ 发布完成：/ipns/{}", result.name);
    }

    Ok(())
}
```

## 核心特性

- ✅ **零依赖部署**: 无需安装 WSL、Docker 或 nargo
- ✅ **跨平台支持**: Windows、Linux、macOS 原生支持
- ✅ **自动环境适配**: 智能选择最佳后端
- ✅ **高性能**: 预编译电路，毫秒级响应
- ✅ **IPNS 管理**: 独立的 IPNS 模块，支持并行发布和 DHT 广播
- ✅ **自动 IPNS 发布**: DID 文档发布时自动发布到 IPNS

## 技术栈

- **密码学**: Ed25519, AES-256-GCM, Blake2s
- **ZKP**: Noir 电路，4 个约束，3-5ms 验证
- **存储**: IPFS 去中心化存储
- **网络**: Iroh P2P 通信
- **命名系统**: IPNS (InterPlanetary Name System)

## 更新记录

### 0.2.15 (最新)
- 🆕 **独立 IPNS 管理模块** - 将 IPNS 功能从 `IpfsClient` 拆分到 `IpnsManager`
- 🚀 **并行发布** - `publish_to_multiple_nodes()` 同时发布到多个节点，速度提升 3 倍
- 📡 **DHT 广播** - `broadcast_to_dht()` 主动触发 DHT 传播
- 📌 **发布 + Pin** - `publish_and_pin()` 确保内容可用性
- ⚡ **预传播** - `prepropagate_cid()` 提前缓存内容，减少首次访问延迟
- 🔧 **依赖修复** - bincode 3.0 是恶作剧版本，降级到 2.0
- 📝 **序列化改进** - 使用 serde_json 替代 bincode

### 0.2.14
- 🚀 快速 IPNS 传播方法
- 📡 DHT 广播辅助功能
- 📢 CID 预传播功能

### 0.2.13
- 🔄 更新 iroh 依赖到 0.96.1
- 📦 更新 directories、cid、multihash 等依赖
- 🔧 修复 iroh API 兼容性问题

### 0.2.11-0.2.12
- 🆕 自动 IPNS 发布功能
- 📡 DHT 直接发布支持
- ⏱️ 30 秒超时保护

## 许可证

MIT License

## 链接

- [GitHub](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)
- [文档](https://docs.rs/diap-rs-sdk)
- [IPNS 管理指南](doc/IPNS_MANAGER.md)
