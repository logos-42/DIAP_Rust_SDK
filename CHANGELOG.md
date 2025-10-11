# DIAP Rust SDK - 更新日志

## [0.1.4] - 2025-01-10

### 🎉 重大变更

**项目重命名**
- ✅ 从 `anp-rs-sdk` 重命名为 `diap-rs-sdk`
- ✅ 完整协议名称：**Decentralized Intelligent Agent Protocol (DIAP)**
- ✅ 所有 API 从 ANP* 更名为 DIAP*
- ✅ 配置路径从 `~/.config/anp-rs-sdk` 更新为 `~/.config/diap-rs-sdk`

**核心结构体重命名**
- `ANPSDK` → `DIAPSDK`
- `ANPConfig` → `DIAPConfig`
- `ANPClient` → `DIAPClient`
- `ANPMessage` → `DIAPMessage`
- `ANPResponse` → `DIAPResponse`
- `ANPKeyGenerator` → `DIAPKeyGenerator`

**文档更新**
- ✅ 全新的 README.md - 重点介绍技术栈和协议逻辑
- ✅ 简化示例代码，突出核心概念
- ✅ 详细的架构图和工作流程说明

### 🔧 技术栈

**身份层**: W3C DID + Ed25519/secp256k1/X25519  
**存储层**: IPFS + IPNS + w3name  
**网络层**: libp2p + Kademlia DHT  
**加密层**: Noise Protocol + AES-GCM  
**协议层**: DIAP Messages

### 📚 文档

- [README.md](README.md) - 项目概览和快速开始
- [config.example.toml](config.example.toml) - 配置文件示例
- [LIBP2P_INTEGRATION_SUMMARY.md](LIBP2P_INTEGRATION_SUMMARY.md) - libp2p 集成总结
- [README_IPFS_IPNS.md](README_IPFS_IPNS.md) - IPFS/IPNS 集成指南

---

## [0.1.3] - 2025-01-08 (旧版本 anp-rs-sdk)

### 🎉 核心功能

**完整的 IPFS/IPNS 集成**
- ✅ IPFS 客户端（支持 AWS IPFS + Pinata）
- ✅ IPNS 发布器（w3name + IPFS 节点）
- ✅ DID 双层验证机制（6步验证流程）
- ✅ 内容上传、获取、Pin 功能

**libp2p P2P 网络支持**
- ✅ libp2p 身份管理
- ✅ PeerID 生成和管理
- ✅ libp2p 节点创建
- ✅ 监听地址配置
- ✅ 节点信息获取

**DID 系统**
- ✅ DID 文档构建器
- ✅ 双层验证发布流程
- ✅ DID 文档更新功能
- ✅ 支持 `did:ipfs`, `did:wba`, `did:web` 格式
- ✅ 批量解析功能

**启动和更新管理**
- ✅ 启动管理器（启动时自动更新 DID）
- ✅ 自动包含最新的 libp2p 信息
- ✅ 序列号递增

**批量操作**
- ✅ 批量上传器
- ✅ 可配置并发数
- ✅ 进度跟踪
- ✅ 详细结果报告

### 📦 新增模块

- `key_manager` - Ed25519 密钥管理
- `ipfs_client` - IPFS 客户端
- `ipns_publisher` - IPNS 发布器
- `did_builder` - DID 文档构建器
- `did_resolver` - DID 解析器
- `libp2p_identity` - libp2p 身份管理
- `libp2p_node` - libp2p 节点
- `p2p_communicator` - P2P 通信
- `startup_manager` - 启动管理
- `batch_uploader` - 批量上传

### 📊 性能指标

- 首次发布: ~5-6秒
- 后续更新: ~1-2秒
- DID 解析: 1-2秒（首次）/ <10ms（缓存）

---

## [0.1.2] - 2025-10-06 (旧版本 anp-rs-sdk)

### 🎉 初始版本

**HTTP 服务器自动配置**
- ✅ 自动端口分配（3000-4000）
- ✅ DID 文档端点：`/.well-known/did.json`
- ✅ 智能体描述端点：`/agents/{id}/ad.json`
- ✅ ANP API 端点：`/anp/api`
- ✅ CORS 支持
- ✅ 健康检查端点

**DID 自动生成**
- ✅ Ed25519 密钥对生成
- ✅ secp256k1 支持
- ✅ X25519 密钥协商
- ✅ 签名和验证 API

**智能体描述文档**
- ✅ 符合 ANP 标准的 AD 文档
- ✅ 自动生成智能体元数据
- ✅ 可配置的接口和能力

**基础示例和文档**
- ✅ 快速开始指南
- ✅ API 文档
- ✅ 配置文件示例

---

## 迁移指南

### 从 anp-rs-sdk 迁移到 diap-rs-sdk

**步骤 1: 更新依赖**

```toml
# 旧版本
[dependencies]
anp-rs-sdk = "0.1.3"

# 新版本
[dependencies]
diap-rs-sdk = "0.1.4"
```

**步骤 2: 更新导入**

```rust
// 旧版本
use anp_rs_sdk::{ANPSDK, ANPConfig, ANPClient};

// 新版本
use diap_rs_sdk::{DIAPSDK, DIAPConfig, DIAPClient};
```

**步骤 3: 更新配置文件路径**

```bash
# 旧路径
~/.config/anp-rs-sdk/config.toml

# 新路径
~/.config/diap-rs-sdk/config.toml
```

**步骤 4: 更新代码**

所有 `ANP*` 结构体和函数都重命名为 `DIAP*`：
- `ANPSDK` → `DIAPSDK`
- `ANPConfig` → `DIAPConfig`
- `ANPClient` → `DIAPClient`
- `ANPMessage` → `DIAPMessage`
- `ANPResponse` → `DIAPResponse`

### API 兼容性

⚠️ **不向后兼容**：由于协议重命名，需要更新所有引用。

✅ **功能完全兼容**：所有功能保持不变，只是名称改变。

---

## 路线图

### v0.2.0（规划中）

- **完整 libp2p Swarm** - NetworkBehaviour、Kademlia DHT、实际连接
- **内容加密** - AES-GCM、混合加密、加密上传
- **Web API 接口** - RESTful API、智能体管理、密钥管理
- **身份认证** - Challenge-Response、会话管理、JWT
- **缓存系统** - DID 解析缓存、TTL 管理

### v0.3.0（规划中）

- **NAT 穿透** - Hole Punching、Relay 支持
- **DHT 完整集成** - 分布式路由表、节点发现
- **多传输协议** - QUIC、WebSocket、WebRTC
- **Web UI** - 控制面板、实时监控、可视化

---

## 贡献

欢迎贡献！请查看 [GitHub Issues](https://github.com/logos-42/DIAP_Rust_SDK/issues)

## 许可证

MIT License


