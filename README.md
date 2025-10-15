# DIAP Rust SDK - Noir ZKP版本

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP (Decentralized Intelligent Agent Protocol)** - 基于Noir零知识证明的去中心化智能体身份协议 Rust SDK

> **🆕 v0.2.5 - 简化架构版**: 专注于Noir ZKP，移除冗余代码，提供完整的IPFS双向验证闭环

## 🎯 核心特性

### ✨ 架构简化对比

| 特性 | 旧版本（v0.2.4） | 新版本（v0.2.5） |
|------|------------------|------------------|
| **ZKP系统** | 双重支持（Noir + Arkworks） | 专注Noir ZKP |
| **代码复杂度** | 高（冗余实现） | 低（精简架构） |
| **依赖数量** | 较多 | 精简 |
| **验证闭环** | 基础验证 | 完整IPFS双向验证 |
| **智能体验证** | 单方验证 | 双向验证系统 |
| **代码质量** | 有警告 | 零警告 |

## 🏗️ 核心架构

### 完整验证闭环

```
┌─────────────────────────────────────────────────────────┐
│                 智能体注册阶段                            │
└─────────────────────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 1. 生成DID密钥对 (sk₁, pk₁)              │
    │    did:key:z6Mk... ← 从pk₁派生           │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 2. 生成libp2p PeerID                     │
    │    12D3Koo... ← 从libp2p密钥派生          │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 3. 构建DID文档并上传到IPFS                │
    │    CID₁ ← IPFS.add(DID文档)              │
    └──────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                IPFS双向验证阶段                          │
└─────────────────────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 发起方智能体A：                           │
    │ 1. 注册到IPFS网络                        │
    │ 2. 发起与智能体B的双向验证               │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 响应方智能体B：                           │
    │ 1. 接收验证请求                          │
    │ 2. 生成Noir ZKP证明                     │
    │ 3. 返回验证结果                          │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 发起方智能体A：                           │
    │ 1. 验证智能体B的证明                     │
    │ 2. 生成自己的Noir ZKP证明               │
    │ 3. 完成双向验证                          │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ ✅ 验证完成：                            │
    │    双方智能体身份已验证，建立信任关系      │
    └──────────────────────────────────────────┘
```

## 🔐 安全设计

### 1. Noir ZKP验证
- **电路约束**: 仅4个约束，高度优化
- **证明大小**: 约192字节
- **验证速度**: 3-5ms
- **隐私保护**: 零知识证明保护私钥信息

### 2. IPFS双向验证
- **去中心化**: 基于IPFS网络，无需中央服务器
- **会话管理**: 自动管理验证会话和过期清理
- **批量验证**: 支持多个智能体同时验证

### 3. 密钥管理
- **Ed25519**: DID身份签名
- **AES-256-GCM**: PeerID加密
- **安全存储**: 加密的密钥备份和恢复

## 🚀 快速开始

### 安装

```toml
[dependencies]
diap-rs-sdk = "0.2.5"
tokio = { version = "1.0", features = ["full"] }
env_logger = "0.10"
```

### 基础示例

```rust
use diap_rs_sdk::*;
use libp2p::identity::Keypair as LibP2PKeypair;
use libp2p::PeerId;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    // 1. 初始化IPFS客户端
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None, None, 30,
    );
    
    // 2. 创建身份管理器（无需ZKP密钥）
    let identity_manager = IdentityManager::new(ipfs_client)?;
    
    // 3. 生成密钥对
    let keypair = KeyPair::generate()?;
    let libp2p_keypair = LibP2PKeypair::generate_ed25519();
    let peer_id = PeerId::from(libp2p_keypair.public());
    
    println!("DID: {}", keypair.did);
    println!("PeerID: {}", peer_id);
    
    // 4. 注册智能体身份
    let agent_info = AgentInfo {
        name: "我的智能体".to_string(),
        services: vec![
            ServiceInfo {
                service_type: "API".to_string(),
                endpoint: serde_json::json!("https://api.example.com"),
            },
        ],
        description: None,
        tags: None,
    };
    
    let registration = identity_manager
        .register_identity(&agent_info, &keypair, &peer_id)
        .await?;
    
    println!("✅ 注册成功！");
    println!("   CID: {}", registration.cid);
    
    Ok(())
}
```

### IPFS双向验证示例

```rust
use diap_rs_sdk::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    // 1. 初始化IPFS双向验证管理器
    let mut verification_manager = IpfsBidirectionalVerificationManager::new(
        "http://localhost:5001".to_string(),
        "http://localhost:8080".to_string(),
    ).await?;
    
    // 2. 注册发起方智能体
    let initiator_keypair = KeyPair::generate()?;
    let initiator_session = verification_manager
        .register_agent(&initiator_keypair, "发起方智能体".to_string())
        .await?;
    
    // 3. 注册响应方智能体
    let responder_keypair = KeyPair::generate()?;
    let responder_session = verification_manager
        .register_agent(&responder_keypair, "响应方智能体".to_string())
        .await?;
    
    // 4. 发起双向验证
    let verification_result = verification_manager
        .initiate_bidirectional_verification(
            initiator_session.session_id.clone(),
            responder_session.session_id.clone(),
        )
        .await?;
    
    println!("✅ 双向验证完成！");
    println!("   发起方验证: {}", verification_result.initiator_verified);
    println!("   响应方验证: {}", verification_result.responder_verified);
    
    Ok(())
}
```

### 运行示例

#### Noir智能体演示
```bash
# 1. 安装Nargo（Noir编译器）
# 在WSL Ubuntu中运行：
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup

# 2. 确保IPFS节点运行
ipfs daemon

# 3. 运行Noir智能体演示
cargo run --example noir_agent_demo
```

#### IPFS双向验证演示
```bash
# 运行完整的IPFS双向验证演示
cargo run --example ipfs_bidirectional_verification_demo
```

## 📦 核心模块

### 1. 密钥管理 (`key_manager`)
- Ed25519密钥对生成
- 密钥备份和恢复
- DID派生（did:key格式）

### 2. DID构建器 (`did_builder`)
- 构建符合W3C DID标准的文档
- 添加加密PeerID服务端点
- 单次上传到IPFS

### 3. 加密PeerID (`encrypted_peer_id`)
- AES-256-GCM加密
- 从Ed25519私钥派生加密密钥
- 安全解密验证

### 4. Noir ZKP系统 (`noir_zkp`, `noir_verifier`)
- **NoirZKPManager**: 管理Noir电路执行
- **NoirVerifier**: 验证Noir ZKP证明
- **智能缓存**: 自动缓存证明结果
- **开发者友好**: 抽象化复杂的Noir/Nargo操作

### 5. 智能体验证 (`agent_verification`)
- 统一的验证接口
- 会话管理和过期清理
- 证明生成和验证

### 6. IPFS双向验证 (`ipfs_bidirectional_verification`)
- **IpfsBidirectionalVerificationManager**: 管理双向验证流程
- **会话管理**: 自动管理验证会话
- **批量验证**: 支持多个智能体同时验证
- **IPFS集成**: 基于IPFS网络的去中心化验证

### 7. 身份管理器 (`identity_manager`)
- 统一的注册、验证接口
- 简化的API设计
- 无需预先生成ZKP密钥

## 📊 性能指标

| 操作 | 性能 | 数据大小 |
|------|------|---------|
| 密钥生成 | <1ms | 32字节 |
| PeerID加密 | <1ms | ~50字节 |
| DID文档构建 | <1ms | ~2KB |
| IPFS上传 | 50-200ms | 取决于网络 |
| Noir ZKP证明生成 | 3-7s (首次) | 192字节 |
| Noir ZKP证明生成 | 0ms (缓存) | 192字节 |
| Noir ZKP证明验证 | 3-5ms | - |
| 双向验证完成 | 6-14s (首次) | - |
| 双向验证完成 | 200ms (缓存) | - |

## 🔧 技术栈

- **密码学**：
  - Ed25519（签名）
  - AES-256-GCM（对称加密）
  - Blake2s（哈希）
  
- **ZKP**：
  - **Noir**：直观的电路描述语言，4个约束
  - Groth16（证明系统）
  - BN254曲线
  
- **存储**：
  - IPFS（去中心化存储）
  - CID（内容寻址）
  
- **网络**：
  - libp2p（P2P通信）
  - PeerID（节点身份）

## 📋 更新日志

### v0.2.5 (2025-01-XX) - 简化架构版

#### 🎯 架构简化
- **专注Noir ZKP**: 移除所有Arkworks相关代码，专注于Noir实现
- **代码精简**: 删除冗余的`zkp_circuit.rs`、`zkp_prover.rs`、`zkp_setup.rs`、`unified_zkp.rs`
- **零警告编译**: 解决所有编译警告，提升代码质量
- **依赖优化**: 精简不必要的依赖，减少编译时间

#### 🚀 新功能
- **完整IPFS双向验证**: 实现智能体之间的双向验证闭环
- **会话管理**: 自动管理验证会话和过期清理
- **批量验证支持**: 支持多个智能体同时验证
- **智能体验证系统**: 完整的智能体身份验证框架

#### 🔧 技术改进
- **Noir电路优化**: 精确匹配Rust和Noir的数据处理逻辑
- **错误处理**: 改进的错误处理和日志记录
- **API简化**: 移除复杂的ZKP密钥预生成要求
- **代码质量**: 解决所有借用检查和类型匹配问题

#### 📚 文档更新
- **README重写**: 全新的文档结构，突出v0.2.5的改进
- **示例更新**: 新增IPFS双向验证示例
- **安装指南**: 简化的安装和使用说明

### v0.2.4 - Noir集成版
- 集成Noir ZKP电路支持
- 实现NoirAgent高级API
- 添加智能缓存系统
- 支持批量处理操作

### v0.2.3 - 生产就绪
- 优化ZKP电路至8个约束
- 完整实现PeerID加密/解密
- 实现安全的密钥备份加密
- 修复所有已知问题

## 🛣️ 路线图

### ✅ v0.2.5 - 简化架构版（当前版本）
- [x] 专注Noir ZKP实现
- [x] 移除所有冗余代码
- [x] 实现完整IPFS双向验证
- [x] 零警告编译
- [x] 精简依赖和API

### 🔮 未来计划
- [ ] 支持多种DID方法（did:web, did:peer等）
- [ ] 实现密钥轮换机制
- [ ] 添加WebAssembly支持
- [ ] 移动端SDK
- [ ] 更多Noir电路优化

## 🤝 贡献

欢迎贡献！请查看 [GitHub Issues](https://github.com/logos-42/DIAP_Rust_SDK/issues)

## 📄 许可证

MIT License - 查看 [LICENSE](LICENSE) 文件

## 🔗 相关链接

- [GitHub 仓库](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)
- [W3C DID 规范](https://www.w3.org/TR/did-core/)
- [Noir 语言](https://noir-lang.org/)
- [IPFS](https://ipfs.io/)

---

**版本**: 0.2.5  
**发布日期**: 2025-01-XX  
**状态**: Simplified Architecture - 简化架构版，专注Noir ZKP和完整IPFS双向验证闭环