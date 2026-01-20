# DIAP Rust SDK

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
diap-rs-sdk = "0.2.12"
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
- ✅ **自动IPNS发布**: DID文档发布时自动发布到IPNS，支持全球访问
- ✅ **DHT传播**: 支持直接发布到DHT网络，确保全球可访问性

## IPNS 自动发布功能

SDK 现在支持在发布 DID 文档时自动发布到 IPNS（InterPlanetary Name System），实现全球可访问的可变指针。

### 使用示例

```rust
use diap_rs_sdk::{IdentityManager, AgentInfo, ServiceInfo, KeyPair, IpfsClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 IPFS 客户端
    let ipfs_client = IpfsClient::new_with_remote_node(
        "http://127.0.0.1:5001".to_string(),
        "http://127.0.0.1:8080".to_string(),
        30
    );
    
    let manager = IdentityManager::new(ipfs_client);
    
    let agent_info = AgentInfo {
        name: "MyAgent".to_string(),
        services: vec![ServiceInfo {
            service_type: "API".to_string(),
            endpoint: serde_json::json!("https://api.example.com"),
        }],
        description: None,
        tags: None,
    };
    
    let keypair = KeyPair::generate()?;
    let node_id = "12D3KooWExampleNodeIdForTesting";
    
    // 注册身份并自动发布到 IPNS
    let registration = manager
        .register_identity_with_ipns(
            &agent_info,
            &keypair,
            node_id,
            Some("my_agent_did"),  // IPNS key 名称
            false,                 // 使用快速发布模式
            Some("8760h"),         // lifetime: 1年
            Some("1h"),            // TTL: 1小时
        )
        .await?;
    
    println!("DID: {}", registration.did);
    println!("CID: {}", registration.cid);
    if let Some(ref ipns_name) = registration.ipns_name {
        println!("IPNS: /ipns/{}", ipns_name);
        println!("全球访问: https://ipfs.io/ipns/{}", ipns_name);
    }
    
    Ok(())
}
```

### 两种发布模式

- **快速发布** (`use_direct_publish=false`): 使用 `allow-offline=true`，立即返回，异步传播到DHT
- **直接发布** (`use_direct_publish=true`): 使用 `allow-offline=false`，要求节点在线，立即尝试传播到DHT

更多详情请参考 [自动IPNS发布指南](doc/AUTO_IPNS_PUBLISHING.md)

## Kubo分支特性（零配置部署）

**适用于**：云服务器、完整节点部署

- ✅ **自动下载安装Kubo**: 首次运行自动下载并安装Kubo (go-ipfs) 二进制文件
- ✅ **智能端口分配**: 自动检测并分配可用端口，避免端口冲突
- ✅ **数据持久化**: 数据存储在 `~/.diap/ipfs`，重启不丢失
- ✅ **完全去中心化**: 运行完整IPFS节点，参与DHT网络
- ✅ **零配置**: 无需手动安装IPFS，开箱即用

## 技术栈

- **密码学**: Ed25519, AES-256-GCM, Blake2s
- **ZKP**: Noir电路，4个约束，3-5ms验证
- **存储**: IPFS去中心化存储
- **网络**: Iroh P2P通信
- **命名系统**: IPNS (InterPlanetary Name System)，支持全球可访问的可变指针

## 更新记录

- 0.2.11
  - 新增：自动IPNS发布功能，DID注册时自动发布到IPNS
  - 新增：`publish_ipns_direct()` 方法，支持直接发布到DHT (allow-offline=false)
  - 新增：`create_and_publish_with_ipns()` 方法，自动发布DID到IPNS
  - 新增：`register_identity_with_ipns()` 方法，注册时自动发布到IPNS
  - 改进：添加30秒超时保护，防止IPNS发布阻塞
  - 改进：扩展 `DIDPublishResult` 和 `IdentityRegistration`，添加IPNS字段
  - 文档：更新示例，集成自动IPNS发布功能

- 0.2.10
  - 新增：Iroh 通信写入 DID 文档（实现 iroh 填写入 DID 文档）
  - 新增：PubSub 解码流程完善并通过验证（pubsub 解码完成验证）
  - 文档：更新安装依赖版本到 `0.2.11`，补充示例

## 许可证

MIT License

## 链接

- [GitHub](https://github.com/logos-42/DIAP_Rust_SDK)
- [Crates.io](https://crates.io/crates/diap-rs-sdk)