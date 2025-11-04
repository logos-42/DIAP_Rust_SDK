# DIAP Rust SDK
# Decentralized Interstellar Agent Protocol
[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

基于零知识证明的去中心化智能体身份协议 Rust SDK，支持跨平台零依赖部署。

## 📦 两个版本

本项目提供两个分支，针对不同的部署场景：

### 🔷 Kubo分支（云服务器版本）
**适用于**：云服务器、完整节点部署

- ✅ 使用Kubo（go-ipfs）作为完整IPFS节点
- ✅ 自动启动和管理本地IPFS守护进程
- ✅ 支持完整的IPFS DHT网络
- ✅ 适合部署在云服务器上
- ✅ 提供最佳的去中心化体验

### 🔷 Helia分支（边缘服务器版本）
**适用于**：边缘计算、资源受限环境

- ✅ 轻量级HTTP客户端，无需本地IPFS守护进程
- ✅ 仅使用HTTP API连接到远程IPFS节点
- ✅ 适合边缘服务器、IoT设备
- ✅ 资源占用小，启动快速
- ✅ 可配置使用公共网关或自定义IPFS节点

> **注意**: 当前分支为 **Helia分支**（轻量级版本）

## 快速开始

### 安装

```toml
[dependencies]
diap-rs-sdk = "0.2.7"
tokio = { version = "1.0", features = ["full"] }
env_logger = "0.10"
```

### 基本使用

```rust
use diap_rs_sdk::{UniversalNoirManager, AgentAuthManager};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    // 1. 创建智能体
    let auth_manager = AgentAuthManager::new().await?;
    let (agent_info, keypair, peer_id) = auth_manager.create_agent("MyAgent", None)?;
    
    println!("智能体创建成功: {}", agent_info.name);
    println!("DID: {}", keypair.did);
    println!("PeerID: {}", peer_id);
    
    // 2. 使用Noir ZKP
    let mut noir_manager = UniversalNoirManager::new().await?;
    
    let inputs = diap_rs_sdk::noir_universal::NoirProverInputs {
        expected_did_hash: "test_hash".to_string(),
        public_key_hash: "pk_hash".to_string(),
        nonce_hash: "nonce_hash".to_string(),
        expected_output: "expected_output".to_string(),
    };
    
    // 生成证明
    let proof = noir_manager.generate_proof(&inputs).await?;
    println!("证明生成成功: {} bytes", proof.proof.len());
    
    // 验证证明
    let result = noir_manager.verify_proof(&proof.proof, &proof.public_inputs).await?;
    println!("验证结果: {}", if result.is_valid { "通过" } else { "失败" });
    
    Ok(())
}
```

### 运行示例

```bash
# 跨平台兼容性演示
cargo run --example cross_platform_demo

# 智能体认证演示
cargo run --example complete_auth_demo

# IPFS双向验证演示
cargo run --example ipfs_bidirectional_verification_demo
```

## 核心特性

- ✅ **零依赖部署**: 无需安装WSL、Docker或nargo
- ✅ **跨平台支持**: Windows、Linux、macOS原生支持
- ✅ **自动环境适配**: 智能选择最佳后端
- ✅ **高性能**: 预编译电路，毫秒级响应
- ✅ **多种后端**: 嵌入、外部、arkworks、简化实现

## Kubo分支特性（零配置部署）

**适用于**：云服务器、完整节点部署

- ✅ **自动下载安装Kubo**: 首次运行自动下载并安装Kubo (go-ipfs) 二进制文件
- ✅ **智能端口分配**: 自动检测并分配可用端口，避免端口冲突
- ✅ **数据持久化**: 数据存储在 `~/.diap/ipfs`，重启不丢失
- ✅ **完全去中心化**: 运行完整IPFS节点，参与DHT网络
- ✅ **零配置**: 无需手动安装IPFS，开箱即用

**首次运行会下载约40MB的Kubo文件，请确保网络连接正常。**

## 技术栈

- **密码学**: Ed25519, AES-256-GCM, Blake2s
- **ZKP**: Noir电路，4个约束，3-5ms验证
- **存储**: IPFS去中心化存储
- **网络**: libp2p, Iroh P2P通信

## 许可证

MIT License

## 链接

- [GitHub](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)
