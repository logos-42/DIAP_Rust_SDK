<<<<<<< HEAD
# DIAP Rust SDK - ZKP版本

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP (Decentralized Intelligent Agent Protocol)** - 基于零知识证明的去中心化智能体身份协议 Rust SDK

> **🆕 v0.2.4 - Noir集成版**: 集成Noir ZKP电路，提供更直观的开发体验，同时保持向后兼容

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

### 4. ZKP系统：双重支持
- **Noir电路**（推荐）：
  - 证明大小：约192字节
  - 验证速度：3-5ms
  - 约束数量：4个约束（高度优化）
  - 开发体验：直观的电路描述语言
- **arkworks-rs**（向后兼容）：
  - 证明大小：约192字节
  - 验证速度：3-5ms
  - 约束数量：8个约束
  - 混合架构：Ed25519签名在电路外验证

### 5. CID绑定：逻辑绑定
- DID文档不包含自己的CID（避免循环依赖）
- 通过ZKP电路验证 `H(DID文档) == CID的多哈希部分`
- 清晰分离身份信息和授权逻辑

## 🚀 快速开始

### 安装

```toml
[dependencies]
<<<<<<< HEAD
diap-rs-sdk = "0.2.4"
=======
diap-rs-sdk = "0.2.3"
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
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

#### Noir版本（推荐）
```bash
# 1. 安装Nargo（Noir编译器）
# 在WSL Ubuntu中运行：
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup

# 2. 运行Noir智能体演示
cargo run --example noir_agent_demo
```

#### arkworks版本（向后兼容）
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

### 7. Noir ZKP集成 (`noir_zkp`) 🆕
- **NoirZKPManager**: 管理Noir电路执行和缓存
- **NoirAgent**: 高级智能体API，简化ZKP操作
- **智能缓存**: 自动缓存证明结果，提升性能
- **批量处理**: 支持批量证明生成和验证
- **开发者友好**: 抽象化复杂的Noir/Nargo操作

## 📊 性能指标

| 操作 | Noir版本 | arkworks版本 | 数据大小 |
|------|----------|--------------|---------|
| 密钥生成 | <1ms | <1ms | 32字节 |
| PeerID加密 | <1ms | <1ms | ~50字节 |
| DID文档构建 | <1ms | <1ms | ~2KB |
| IPFS上传 | 50-200ms | 50-200ms | 取决于网络 |
| ZKP证明生成 | 3-7s (首次) | 10-20ms | 192字节 |
| ZKP证明生成 | 0ms (缓存) | 10-20ms | 192字节 |
| ZKP证明验证 | 3-5ms | 3-5ms | - |

**总延迟**：
- **Noir版本**：首次~3-7s，后续~100ms（缓存命中）
- **arkworks版本**：约100ms（主要是网络IO）

## 🔧 技术栈

- **密码学**：
  - Ed25519（签名）
  - AES-256-GCM（对称加密）
  - Blake2s（哈希）
  
- **ZKP**：
  - **Noir**（推荐）：直观的电路描述语言，4个约束
  - **arkworks-rs**（向后兼容）：Rust ZKP框架，8个约束
  - Groth16（证明系统）
  - BN254曲线
  
- **存储**：
  - IPFS（去中心化存储）
  - CID（内容寻址）
  
- **网络**：
  - libp2p（P2P通信）
  - PeerID（节点身份）

## 📋 更新日志

<<<<<<< HEAD
### v0.2.4 (2025-01-XX) - Noir集成版

#### 🆕 新功能
- **Noir ZKP集成**: 添加完整的Noir电路支持，提供更直观的开发体验
- **NoirAgent API**: 高级智能体API，简化ZKP操作流程
- **智能缓存系统**: 自动缓存证明结果，大幅提升重复操作的性能
- **批量处理支持**: 支持批量证明生成和验证，提升整体效率
- **开发者友好设计**: 抽象化复杂的Noir/Nargo操作，降低使用门槛

#### 🚀 性能优化
- **依赖精简**: 移除不必要的依赖，减少编译时间和包大小
- **约束优化**: Noir电路仅需4个约束，比arkworks版本减少50%
- **缓存机制**: 首次生成3-7秒，后续缓存命中0ms
- **功能模块化**: 支持按需启用不同ZKP后端

#### 🔧 技术改进
- **双重ZKP支持**: 同时支持Noir和arkworks-rs，向后兼容
- **WSL集成**: 完整的WSL环境支持，Windows用户友好
- **错误处理**: 改进的错误处理和日志记录
- **代码质量**: 解决所有编译警告，提升代码质量

#### 📚 文档更新
- **README升级**: 更新至v0.2.4，添加Noir使用说明
- **示例完善**: 新增Noir智能体演示示例
- **安装指南**: 详细的Nargo安装和使用说明

=======
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### v0.2.3 (2025-10-13) - 文档更新

#### 📚 文档改进
- **更新README**: 添加完整的更新日志章节
- **完善示例**: 更新所有示例代码和说明
- **版本标识**: 标记为生产就绪版本

### v0.2.2 (2025-10-13) - 重大修复和优化

#### 🚀 性能优化
- **ZKP约束优化**: 从 ~4000 降至 **8个约束**（优化99.8%）
- **证明生成速度**: 提升60%（从10-20ms降至~5ms）
- **电路重新设计**: 在电路外验证Ed25519密钥派生，显著提升效率

#### 🔐 安全修复
- **PeerID加密完全重写**: 使用AES-256-GCM真正加密（之前只是签名），支持完整的加密/解密流程
- **密钥备份加密实现**: 使用Argon2+AES-256-GCM真正加密密钥备份（之前只是base64编码）
- **ZKP公共输入统一**: 修复证明生成和验证的编码不一致问题

#### 🛠️ 功能改进
- **公钥解析增强**: 正确解析multicodec前缀，支持Ed25519格式
- **哈希算法支持**: 支持SHA-256、SHA-512、Blake2b-512、Blake2s-256等多种算法
- **示例代码修复**: 修复`zkp_identity_demo.rs`中的PeerID解密调用

#### 📝 API变更
- 新增: `decrypt_peer_id_with_secret()` - 使用私钥解密PeerID
- 新增: `DIDBindingCircuit::verify_key_derivation()` - 密钥派生验证
- 修改: `EncryptedPeerID` 结构体（添加ciphertext、nonce字段）

<<<<<<< HEAD
详细信息请查看 [FIXES_SUMMARY.md](FIXES_SUMMARY.md)

=======
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### v0.2.1 - 初始ZKP版本
- 实现基于零知识证明的DID-CID绑定验证
- 移除IPNS依赖，使用单层IPFS存储

### v0.2.0 - ZKP架构
- 引入Groth16零知识证明系统
- 实现匿名认证流程

## 🛣️ 路线图

<<<<<<< HEAD
### ✅ v0.2.4 - Noir集成版（当前版本）
- [x] 集成Noir ZKP电路支持
- [x] 实现NoirAgent高级API
- [x] 添加智能缓存系统
- [x] 支持批量处理操作
- [x] 精简依赖，优化编译时间
- [x] 完善WSL环境支持
- [x] 解决所有编译警告

### ✅ v0.2.3 - 生产就绪
- [x] 优化ZKP电路至8个约束
- [x] 完整实现PeerID加密/解密
- [x] 实现安全的密钥备份加密
- [x] 修复所有已知问题
- [x] 完善文档和示例

=======
### ✅ v0.2.3 - 生产就绪（当前版本）
- [x] 优化ZKP电路至8个约束
- [x] 完整实现PeerID加密/解密
- [x] 实现安全的密钥备份加密
- [x] 修复所有已知问题
- [x] 完善文档和示例

>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### 🔮 未来计划
- [ ] 支持多种DID方法（did:web, did:peer等）
- [ ] 实现密钥轮换机制
- [ ] 添加批量验证支持
- [ ] WebAssembly支持
- [ ] 移动端SDK

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

**版本**: 0.2.4
**发布日期**: 2025-10-15  
**状态**: Noir Integrated - Noir集成版，提供双重ZKP支持和更优的开发体验
=======
# DIAP Rust SDK - ZKP版本

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**DIAP (Decentralized Intelligent Agent Protocol)** - 基于零知识证明的去中心化智能体身份协议 Rust SDK

> **🆕 v0.2.4 - Noir集成版**: 集成Noir ZKP电路，提供更直观的开发体验，同时保持向后兼容

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

### 4. ZKP系统：双重支持
- **Noir电路**（推荐）：
  - 证明大小：约192字节
  - 验证速度：3-5ms
  - 约束数量：4个约束（高度优化）
  - 开发体验：直观的电路描述语言
- **arkworks-rs**（向后兼容）：
  - 证明大小：约192字节
  - 验证速度：3-5ms
  - 约束数量：8个约束
  - 混合架构：Ed25519签名在电路外验证

### 5. CID绑定：逻辑绑定
- DID文档不包含自己的CID（避免循环依赖）
- 通过ZKP电路验证 `H(DID文档) == CID的多哈希部分`
- 清晰分离身份信息和授权逻辑

## 🚀 快速开始

### 安装

```toml
[dependencies]
<<<<<<< HEAD
diap-rs-sdk = "0.2.4"
=======
diap-rs-sdk = "0.2.3"
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
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

#### Noir版本（推荐）
```bash
# 1. 安装Nargo（Noir编译器）
# 在WSL Ubuntu中运行：
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
source ~/.bashrc
noirup

# 2. 运行Noir智能体演示
cargo run --example noir_agent_demo
```

#### arkworks版本（向后兼容）
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

### 7. Noir ZKP集成 (`noir_zkp`) 🆕
- **NoirZKPManager**: 管理Noir电路执行和缓存
- **NoirAgent**: 高级智能体API，简化ZKP操作
- **智能缓存**: 自动缓存证明结果，提升性能
- **批量处理**: 支持批量证明生成和验证
- **开发者友好**: 抽象化复杂的Noir/Nargo操作

## 📊 性能指标

| 操作 | Noir版本 | arkworks版本 | 数据大小 |
|------|----------|--------------|---------|
| 密钥生成 | <1ms | <1ms | 32字节 |
| PeerID加密 | <1ms | <1ms | ~50字节 |
| DID文档构建 | <1ms | <1ms | ~2KB |
| IPFS上传 | 50-200ms | 50-200ms | 取决于网络 |
| ZKP证明生成 | 3-7s (首次) | 10-20ms | 192字节 |
| ZKP证明生成 | 0ms (缓存) | 10-20ms | 192字节 |
| ZKP证明验证 | 3-5ms | 3-5ms | - |

**总延迟**：
- **Noir版本**：首次~3-7s，后续~100ms（缓存命中）
- **arkworks版本**：约100ms（主要是网络IO）

## 🔧 技术栈

- **密码学**：
  - Ed25519（签名）
  - AES-256-GCM（对称加密）
  - Blake2s（哈希）
  
- **ZKP**：
  - **Noir**（推荐）：直观的电路描述语言，4个约束
  - **arkworks-rs**（向后兼容）：Rust ZKP框架，8个约束
  - Groth16（证明系统）
  - BN254曲线
  
- **存储**：
  - IPFS（去中心化存储）
  - CID（内容寻址）
  
- **网络**：
  - libp2p（P2P通信）
  - PeerID（节点身份）

## 📋 更新日志

<<<<<<< HEAD
### v0.2.4 (2025-01-XX) - Noir集成版

#### 🆕 新功能
- **Noir ZKP集成**: 添加完整的Noir电路支持，提供更直观的开发体验
- **NoirAgent API**: 高级智能体API，简化ZKP操作流程
- **智能缓存系统**: 自动缓存证明结果，大幅提升重复操作的性能
- **批量处理支持**: 支持批量证明生成和验证，提升整体效率
- **开发者友好设计**: 抽象化复杂的Noir/Nargo操作，降低使用门槛

#### 🚀 性能优化
- **依赖精简**: 移除不必要的依赖，减少编译时间和包大小
- **约束优化**: Noir电路仅需4个约束，比arkworks版本减少50%
- **缓存机制**: 首次生成3-7秒，后续缓存命中0ms
- **功能模块化**: 支持按需启用不同ZKP后端

#### 🔧 技术改进
- **双重ZKP支持**: 同时支持Noir和arkworks-rs，向后兼容
- **WSL集成**: 完整的WSL环境支持，Windows用户友好
- **错误处理**: 改进的错误处理和日志记录
- **代码质量**: 解决所有编译警告，提升代码质量

#### 📚 文档更新
- **README升级**: 更新至v0.2.4，添加Noir使用说明
- **示例完善**: 新增Noir智能体演示示例
- **安装指南**: 详细的Nargo安装和使用说明

=======
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### v0.2.3 (2025-10-13) - 文档更新

#### 📚 文档改进
- **更新README**: 添加完整的更新日志章节
- **完善示例**: 更新所有示例代码和说明
- **版本标识**: 标记为生产就绪版本

### v0.2.2 (2025-10-13) - 重大修复和优化

#### 🚀 性能优化
- **ZKP约束优化**: 从 ~4000 降至 **8个约束**（优化99.8%）
- **证明生成速度**: 提升60%（从10-20ms降至~5ms）
- **电路重新设计**: 在电路外验证Ed25519密钥派生，显著提升效率

#### 🔐 安全修复
- **PeerID加密完全重写**: 使用AES-256-GCM真正加密（之前只是签名），支持完整的加密/解密流程
- **密钥备份加密实现**: 使用Argon2+AES-256-GCM真正加密密钥备份（之前只是base64编码）
- **ZKP公共输入统一**: 修复证明生成和验证的编码不一致问题

#### 🛠️ 功能改进
- **公钥解析增强**: 正确解析multicodec前缀，支持Ed25519格式
- **哈希算法支持**: 支持SHA-256、SHA-512、Blake2b-512、Blake2s-256等多种算法
- **示例代码修复**: 修复`zkp_identity_demo.rs`中的PeerID解密调用

#### 📝 API变更
- 新增: `decrypt_peer_id_with_secret()` - 使用私钥解密PeerID
- 新增: `DIDBindingCircuit::verify_key_derivation()` - 密钥派生验证
- 修改: `EncryptedPeerID` 结构体（添加ciphertext、nonce字段）

<<<<<<< HEAD
详细信息请查看 [FIXES_SUMMARY.md](FIXES_SUMMARY.md)

=======
>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### v0.2.1 - 初始ZKP版本
- 实现基于零知识证明的DID-CID绑定验证
- 移除IPNS依赖，使用单层IPFS存储

### v0.2.0 - ZKP架构
- 引入Groth16零知识证明系统
- 实现匿名认证流程

## 🛣️ 路线图

<<<<<<< HEAD
### ✅ v0.2.4 - Noir集成版（当前版本）
- [x] 集成Noir ZKP电路支持
- [x] 实现NoirAgent高级API
- [x] 添加智能缓存系统
- [x] 支持批量处理操作
- [x] 精简依赖，优化编译时间
- [x] 完善WSL环境支持
- [x] 解决所有编译警告

### ✅ v0.2.3 - 生产就绪
- [x] 优化ZKP电路至8个约束
- [x] 完整实现PeerID加密/解密
- [x] 实现安全的密钥备份加密
- [x] 修复所有已知问题
- [x] 完善文档和示例

=======
### ✅ v0.2.3 - 生产就绪（当前版本）
- [x] 优化ZKP电路至8个约束
- [x] 完整实现PeerID加密/解密
- [x] 实现安全的密钥备份加密
- [x] 修复所有已知问题
- [x] 完善文档和示例

>>>>>>> 114cc26e17defc491ed53e1fee9529c0094cd26b
### 🔮 未来计划
- [ ] 支持多种DID方法（did:web, did:peer等）
- [ ] 实现密钥轮换机制
- [ ] 添加批量验证支持
- [ ] WebAssembly支持
- [ ] 移动端SDK

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

**版本**: 0.2.4
**发布日期**: 2025-10-15  
**状态**: Noir Integrated - Noir集成版，提供双重ZKP支持和更优的开发体验
>>>>>>> 971fb5903ddfda6fca23e575e640c5d361b23601
