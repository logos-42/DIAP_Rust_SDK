# DIAP Rust SDK - ZKP版本

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP (Decentralized Intelligent Agent Protocol)** - 基于零知识证明的去中心化智能体身份协议 Rust SDK

> **🆕 v0.2.2 - ZKP优化版**: 使用零知识证明验证DID-CID绑定，移除IPNS依赖，大幅简化架构

## 🎯 核心特性

### ✨ 与传统方案的对比

| 特性 | 传统方案（v0.1.x） | ZKP方案（v0.2.0+） |
|------|------------------|-------------------|
| **身份格式** | `did:ipfs:<ipns_name>` | `did:key:<public_key>` |
| **存储依赖** | IPFS + IPNS双层 | 仅IPFS单层 |
| **验证方式** | IPNS记录解析 | ZKP密码学证明 |
| **上传次数** | 2次（双层验证） | 1次（单次上传） |
| **PeerID保护** | 明文存储 | 私钥加密存储 |
| **去中心化程度** | 依赖IPNS网络 | 完全去中心化 |
| **匿名性** | 部分匿名 | 强匿名性 |

## 🏗️ 核心架构

### 工作流程

```
┌─────────────────────────────────────────────────────────┐
│                 系统初始化（静态加载）                      │
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
    │ 3. 加密PeerID                            │
    │    encrypted_peer_id ← E_sk₁(PeerID)     │
    │    使用AES-256-GCM加密                   │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 4. 构建DID文档                           │
    │    包含: pk₁, encrypted_peer_id          │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 5. 上传到IPFS（一次性）                   │
    │    CID₁ ← IPFS.add(DID文档)              │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ ✅ 信任根确立：                          │
    │    DID ←→ CID 通过ZKP绑定                │
    └──────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│               匿名认证流程（动态加载）                      │
└─────────────────────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 用户：生成临时PeerID_temp                 │
    │       （隐藏真实身份）                     │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 用户 → IPN：请求访问资源                  │
    │       发送: CID₁, PeerID_temp            │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ IPN → 用户：返回挑战 nonce                │
    │       nonce = Hash(IPN_PeerID, temp_PeerID) │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 用户：生成ZKP证明                         │
    │   π ← Prove(sk₁, DID文档, nonce, CID₁)  │
    │   证明逻辑：                              │
    │     ✓ H(DID文档) == CID₁                 │
    │     ✓ sk₁派生出pk₁                       │
    │     ✓ pk₁在DID文档中                     │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ 用户 → IPN：提交证明                      │
    │       发送: π, CID₁, nonce              │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ IPN：验证ZKP证明                          │
    │   1. 从IPFS获取CID₁对应的DID文档          │
    │   2. 验证π的有效性                        │
    │   3. 验证nonce防重放                      │
    └──────────────────────────────────────────┘
                          ↓
    ┌──────────────────────────────────────────┐
    │ ✅ IPN：授权访问                          │
    │    用户身份已验证，允许访问资源            │
    └──────────────────────────────────────────┘
```

## 🔐 安全设计

### 1. 隐私模型：弱匿名性
- **主DID**：公开可查，用于审计和信任建立
- **临时PeerID**：每次会话使用一次性PeerID，网络层行为不可关联
- **加密PeerID**：真实PeerID使用DID私钥加密，只有持有者能解密

### 2. 防重放攻击：挑战-响应
- 每次认证使用新的nonce
- nonce绑定双方PeerID
- ZKP证明包含nonce验证

### 3. 密钥管理
- **Ed25519**：DID身份签名
- **AES-256-GCM**：PeerID加密
- 支持密钥轮换和恢复

### 4. ZKP系统：Groth16
- **证明大小**：约192字节
- **验证速度**：3-5ms
- **约束数量**：约4000（优化后）
- **混合架构**：Ed25519签名在电路外验证

### 5. CID绑定：逻辑绑定
- DID文档不包含自己的CID（避免循环依赖）
- 通过ZKP电路验证 `H(DID文档) == CID的多哈希部分`
- 清晰分离身份信息和授权逻辑

## 🚀 快速开始

### 安装

```toml
[dependencies]
diap-rs-sdk = "0.2.2"
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
    
    // 1. 初始化
    let ipfs_client = IpfsClient::new(
        Some("http://localhost:5001".to_string()),
        Some("http://localhost:8080".to_string()),
        None, None, 30,
    );
    
    // 加载ZKP keys（需先运行 zkp_setup_keys 生成）
    let identity_manager = IdentityManager::new_with_keys(
        ipfs_client,
        "zkp_proving.key",
        "zkp_verifying.key",
    )?;
    
    // 2. 生成密钥
    let keypair = KeyPair::generate()?;
    let libp2p_keypair = LibP2PKeypair::generate_ed25519();
    let peer_id = PeerId::from(libp2p_keypair.public());
    
    println!("DID: {}", keypair.did);
    println!("PeerID: {}", peer_id);
    
    // 3. 注册身份
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
    
    // 4. 生成ZKP证明
    let nonce = b"challenge_from_resource_node";
    let proof = identity_manager.generate_binding_proof(
        &keypair,
        &registration.did_document,
        &registration.cid,
        nonce,
    )?;
    
    println!("✅ ZKP证明生成");
    
    // 5. 验证身份
    let verification = identity_manager.verify_identity_with_zkp(
        &registration.cid,
        &proof.proof,
        nonce,
    ).await?;
    
    println!("✅ 验证结果: {}", verification.zkp_verified);
    
    Ok(())
}
```

### 运行示例

```bash
# 1. 首先生成ZKP可信设置（proving key和verifying key）
cargo run --example zkp_setup_keys

# 2. 确保IPFS节点运行在 localhost:5001
ipfs daemon

# 3. 运行ZKP身份演示
cargo run --example zkp_identity_demo
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

### 4. ZKP电路 (`zkp_circuit`)
- DID-CID绑定证明电路
- Blake2s哈希验证（约2500约束）
- 密钥派生验证（约1000约束）

### 5. ZKP证明器 (`zkp_prover`)
- Groth16证明生成
- 证明验证
- 可信设置管理

### 6. 身份管理器 (`identity_manager`)
- 统一的注册、验证接口
- ZKP证明生成和验证
- PeerID加解密

## 📊 性能指标

| 操作 | 时间 | 数据大小 |
|------|------|---------|
| 密钥生成 | <1ms | 32字节 |
| PeerID加密 | <1ms | ~50字节 |
| DID文档构建 | <1ms | ~2KB |
| IPFS上传 | 50-200ms | 取决于网络 |
| ZKP证明生成 | 10-20ms | 192字节 |
| ZKP证明验证 | 3-5ms | - |

**总延迟**：约100ms（主要是网络IO）

## 🔧 技术栈

- **密码学**：
  - Ed25519（签名）
  - AES-256-GCM（对称加密）
  - Blake2s（哈希）
  
- **ZKP**：
  - arkworks-rs（ZKP框架）
  - Groth16（证明系统）
  - BN254曲线
  
- **存储**：
  - IPFS（去中心化存储）
  - CID（内容寻址）
  
- **网络**：
  - libp2p（P2P通信）
  - PeerID（节点身份）

## 🛣️ 路线图

### ✅ v0.2.2 - ZKP优化（当前版本）
- [x] 移除IPNS依赖
- [x] 实现PeerID加密
- [x] 实现ZKP电路
- [x] 实现证明生成/验证
- [x] 简化DID文档结构
- [x] 优化代码结构和文档


## 🤝 贡献

欢迎贡献！请查看 [GitHub Issues](https://github.com/logos-42/DIAP_Rust_SDK/issues)

## 📄 许可证

MIT License - 查看 [LICENSE](LICENSE) 文件

## 🔗 相关链接

- [GitHub 仓库](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)
- [W3C DID 规范](https://www.w3.org/TR/did-core/)
- [arkworks ZKP](https://github.com/arkworks-rs)
- [Groth16论文](https://eprint.iacr.org/2016/260.pdf)

---

**版本**: 0.2.2
**发布日期**: 2025-10-13  
**状态**: Beta - ZKP核心功能完整，适合开发使用
