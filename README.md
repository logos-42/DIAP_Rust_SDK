# DIAP Rust SDK

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP (Decentralized Intelligent Agent Protocol)** 是一个完整的去中心化智能体协议 Rust SDK，提供了构建去中心化智能体所需的全部基础设施。

> **🆕 最新版本 v0.1.4**: 完整的去中心化智能体协议实现，包括 libp2p P2P 网络、IPFS/IPNS 存储、DID 身份认证

## 🎯 什么是 DIAP？

DIAP 是一个去中心化智能体协议，旨在让智能体能够：
- **自主身份**：通过 DID（去中心化标识符）拥有独立的数字身份
- **P2P 通信**：通过 libp2p 实现点对点直连，无需中心化服务器
- **永久存储**：通过 IPFS/IPNS 实现内容寻址和可更新的去中心化存储
- **安全互操作**：基于密码学验证的端到端加密通信

## 🏗️ 核心架构

### 协议层次结构

```
┌─────────────────────────────────────────────────────┐
│                   应用层                              │
│           (智能体业务逻辑)                             │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│                DIAP 协议层                            │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐       │
│  │ DID 身份  │  │ 消息协议   │  │ 服务发现  │       │
│  └───────────┘  └───────────┘  └───────────┘       │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│              去中心化基础设施层                        │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐       │
│  │  libp2p   │  │ IPFS/IPNS │  │   DHT     │       │
│  │  P2P网络  │  │ 内容存储   │  │  路由发现  │       │
│  └───────────┘  └───────────┘  └───────────┘       │
└─────────────────────────────────────────────────────┘
                        ↓
┌─────────────────────────────────────────────────────┐
│                  传输层                               │
│         (TCP/IP, QUIC, WebRTC等)                     │
└─────────────────────────────────────────────────────┘
```

### 工作流程

```
智能体 A 想要与智能体 B 通信：

1️⃣ 身份发现
   A 拥有 B 的 DID → 通过 IPNS 解析 B 的 DID 文档
   
2️⃣ 地址解析
   从 DID 文档提取 B 的 libp2p PeerID 和网络地址
   
3️⃣ 身份验证
   验证 PeerID 与 DID 文档的一致性（双层验证）
   
4️⃣ 建立连接
   使用 libp2p 建立 P2P 加密通道（Noise 协议）
   
5️⃣ 安全通信
   在加密通道中交换 DIAP 协议消息
```

## 🔑 核心技术栈

### 1. 身份层 (DID)

**技术选型**：W3C DID 标准 + 自定义扩展

**实现方式**：
- `did:ipfs:<k51...>` - 基于 IPNS 名称的 DID
- `did:web:<domain>` - 基于域名的 DID（兼容）
- Ed25519 数字签名算法
- secp256k1 曲线（兼容以太坊）
- X25519 密钥协商（端到端加密）

**关键特性**：
- DID 文档包含公钥、服务端点、验证方法
- 支持 JWK 和 Multibase 两种公钥格式
- 密钥轮转和恢复机制

### 2. 存储层 (IPFS/IPNS)

**技术选型**：IPFS + IPNS + w3name

**为什么选择 IPFS？**
- **内容寻址**：通过内容哈希（CID）保证数据完整性
- **去中心化**：无单点故障，数据分布式存储
- **可验证**：CID 可加密验证内容未被篡改

**IPNS 的作用**：
- IPFS 的 CID 是不可变的（内容变化 CID 就变化）
- IPNS 提供可更新的指针：`/ipns/<k51...>` → `/ipfs/<CID>`
- 智能体更新 DID 文档时，只需更新 IPNS 记录

**双层验证机制**：
```
DID 文档包含：
  - DID: did:ipfs:k51qzi5uqu5...
  - IPNS 名称: k51qzi5uqu5...
  - 当前 CID: bafybeid...

验证流程：
  1. 从 DID 提取 IPNS 名称
  2. 解析 IPNS → 获得最新 CID
  3. 从 IPFS 获取 CID 对应的 DID 文档
  4. 验证文档中的 IPNS 名称与 DID 一致
  ✅ 形成验证闭环，防止伪造
```

### 3. 网络层 (libp2p)

**技术选型**：libp2p + Kademlia DHT

**为什么选择 libp2p？**
- **模块化设计**：可插拔的传输层、加密层、多路复用
- **NAT 穿透**：支持多种打洞技术（Hole Punching, Relay）
- **多传输协议**：TCP、QUIC、WebSocket、WebRTC
- **成熟生态**：IPFS、Filecoin、Polkadot 都基于 libp2p

**核心组件**：
- **Transport**: TCP/QUIC 传输
- **Noise**: 加密握手协议（替代 TLS）
- **Yamux**: 单连接多路复用
- **Kademlia DHT**: 分布式路由表，节点发现
- **mDNS**: 本地网络自动发现

**PeerID 与 DID 的关系**：
```
libp2p PeerID = Hash(libp2p公钥)
DID = Hash(IPNS私钥)

智能体拥有两个密钥对：
1. IPNS 密钥对 → DID 身份
2. libp2p 密钥对 → P2P 通信

DID 文档将两者绑定：
  "id": "did:ipfs:k51..."
  "verificationMethod": [
    { "publicKeyMultibase": "<IPNS公钥>" },
    { "publicKeyMultibase": "<libp2p公钥>" }
  ]
```

### 4. 加密层

**密钥算法选择**：
- **Ed25519**: 
  - 签名速度快（~60K 签名/秒）
  - 公钥 32 字节，签名 64 字节
  - 用于 DID 身份签名、IPNS 记录签名
  
- **secp256k1**: 
  - 与以太坊兼容
  - 用于跨链身份互操作
  
- **X25519**: 
  - ECDH 密钥协商
  - 用于端到端加密（规划中）
  
- **AES-GCM**: 
  - 对称加密，高性能
  - 用于内容加密（规划中）

**Noise 协议**：
- libp2p 默认加密协议
- 类似 TLS 但更轻量
- 提供前向保密（Forward Secrecy）

### 5. 协议层 (DIAP Messages)

**消息格式**：
```json
{
  "msg_type": "request|response|event",
  "from": "did:ipfs:k51...",
  "to": "did:ipfs:k52...",
  "content": { /* 业务数据 */ },
  "timestamp": "2025-01-10T10:00:00Z",
  "nonce": "random_string",  // 防重放攻击
  "signature": "base64..."   // 发送者签名
}
```

**安全特性**：
- 每条消息都包含时间戳和 nonce
- 接收方验证签名和时间窗口
- 防止重放攻击和中间人攻击

## 🚀 快速开始

### 安装

```toml
[dependencies]
diap-rs-sdk = "0.1.4"
tokio = { version = "1.0", features = ["full"] }
```

### 最小示例

```rust
use diap_rs_sdk::{DIAPSDK, AutoConfigOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建智能体
    let mut sdk = DIAPSDK::new(AutoConfigOptions::default());
    
    // 一键启动
    let config = sdk.start().await?;
    
    println!("✅ 智能体已启动");
    println!("   DID: {}", config.did);
    println!("   端点: {}", config.endpoint);
    
    // 保持运行
    tokio::signal::ctrl_c().await?;
    sdk.stop().await?;
    Ok(())
}
```

### 运行示例

```bash
# 基础示例
cargo run --example basic_agent_with_did_web

# 完整功能（需要 IPFS 节点）
cargo run --example complete_agent_with_ipfs

# P2P 通信演示
cargo run --example p2p_communication_demo

# DID 解析演示
cargo run --example did_resolver_demo
```

## 📊 功能状态

### ✅ 已实现

| 模块 | 功能 | 说明 |
|------|------|------|
| **密钥管理** | Ed25519, secp256k1, X25519 | 密钥生成、存储、备份 |
| **DID 系统** | did:ipfs, did:web | DID 文档生成、发布、解析 |
| **IPFS 集成** | 上传、获取、Pin | 支持 AWS IPFS、Pinata |
| **IPNS 发布** | w3name + IPFS 节点 | IPNS 记录发布和解析 |
| **双层验证** | DID ↔ IPNS ↔ CID | 完整验证闭环 |
| **libp2p 节点** | 节点创建、监听 | PeerID 生成、多地址 |
| **DID 解析** | 多格式支持 | 批量解析、多源回退 |
| **HTTP 服务** | 自动配置 | DID 文档、AD 文档端点 |

### 🚧 规划中

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **完整 Swarm** | libp2p NetworkBehaviour | 高 |
| **内容加密** | AES-GCM, 公钥加密 | 高 |
| **DHT 集成** | Kademlia 路由 | 中 |
| **NAT 穿透** | Hole Punching, Relay | 中 |
| **缓存系统** | DID 解析缓存 | 低 |
| **Web UI** | 控制面板 | 低 |

## 🔧 配置说明

创建配置文件 `~/.config/diap-rs-sdk/config.toml`:

```toml
[agent]
name = "My DIAP Agent"
private_key_path = "~/.local/share/diap-rs-sdk/keys/agent.key"
auto_generate_key = true

[ipfs]
aws_api_url = "http://your-ipfs-node:5001"
aws_gateway_url = "http://your-ipfs-node:8080"
timeout_seconds = 30

[ipns]
use_w3name = true
use_ipfs_node = true
validity_days = 365

[libp2p]
listen_addresses = ["/ip4/0.0.0.0/tcp/4001"]

[http]
auto_port = true
port_range_start = 3000
port_range_end = 4000
```

## 📚 技术文档

详细文档请查看：
- [IPFS/IPNS 集成指南](README_IPFS_IPNS.md)
- [libp2p 集成总结](LIBP2P_INTEGRATION_SUMMARY.md)
- [API 文档](https://docs.rs/diap-rs-sdk)

## 🌟 为什么选择 DIAP？

### 真正的去中心化

- ❌ 不依赖任何中心化服务器
- ✅ 数据存储在 IPFS 分布式网络
- ✅ 点对点直连，无中间商

### 安全可验证

- 所有通信基于密码学验证
- DID 文档内容寻址，防篡改
- 端到端加密（规划中）

### 互操作性

- 兼容 W3C DID 标准
- 支持多种 DID 格式
- 基于开放协议（libp2p, IPFS）

### 高性能

- Rust 实现，零成本抽象
- 异步 IO（Tokio）
- 批量操作、连接池

## 🤝 贡献

欢迎贡献！请查看 [GitHub Issues](https://github.com/logos-42/DIAP_Rust_SDK/issues)

## 📄 许可证

MIT License - 查看 [LICENSE](LICENSE) 文件

## 🔗 相关链接

- [GitHub 仓库](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)
- [W3C DID 规范](https://www.w3.org/TR/did-core/)
- [libp2p 文档](https://libp2p.io/)
- [IPFS 文档](https://docs.ipfs.tech/)

---

**版本**: 0.1.4  
**发布日期**: 2025-01-10  
**状态**: Beta - 核心功能完整，适合开发使用
