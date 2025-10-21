# DIAP Rust SDK

[![Crates.io](https://img.shields.io/crates/v/diap-rs-sdk.svg)](https://crates.io/crates/diap-rs-sdk)
[![Documentation](https://docs.rs/diap-rs-sdk/badge.svg)](https://docs.rs/diap-rs-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

基于零知识证明的去中心化智能体身份协议 Rust SDK，支持跨平台零依赖部署。

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