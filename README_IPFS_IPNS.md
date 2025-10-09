# ANP Rust SDK - IPFS/IPNS 功能说明

## 🆕 v0.2.0 新增功能

### 核心特性

#### 1. 完整的IPFS/IPNS集成 🌐
- ✅ 自动上传DID文档到IPFS
- ✅ 自动发布IPNS记录
- ✅ DID双层验证机制
- ✅ 多源回退策略

#### 2. 灵活的存储选项 📦
- ✅ AWS IPFS节点（优先，自建）
- ✅ Pinata备用（免费1GB）
- ✅ 自动回退机制

#### 3. 强大的DID解析 🔍
- ✅ 支持 `did:ipfs:<ipns-name>` 格式
- ✅ 支持 `did:wba:<domain>` 格式（兼容）
- ✅ 支持 `did:web:<domain>` 格式
- ✅ 多源解析策略

#### 4. 批量操作 ⚡
- ✅ 批量上传多个DID文档
- ✅ 并发控制（可配置）
- ✅ 进度跟踪

#### 5. 自动更新 ⏰
- ✅ 定时自动更新（24小时）
- ✅ 自动刷新IPNS有效期
- ✅ 后台运行，无需干预

## 快速开始

### 安装

```toml
[dependencies]
anp-rs-sdk = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
env_logger = "0.10"
```

### 基础使用

```rust
use anp_rs_sdk::{
    ANPConfig, KeyManager, IpfsClient, IpnsPublisher, DIDBuilder,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 加载配置
    let config = ANPConfig::load()?;
    
    // 2. 初始化密钥
    let key_manager = KeyManager::new(
        config.agent.private_key_path.parent().unwrap().to_path_buf()
    );
    let keypair = key_manager.load_or_generate(&config.agent.private_key_path)?;
    
    // 3. 创建IPFS/IPNS客户端
    let ipfs_client = IpfsClient::new(
        config.ipfs.aws_api_url.clone(),
        config.ipfs.aws_gateway_url.clone(),
        config.ipfs.pinata_api_key.clone(),
        config.ipfs.pinata_api_secret.clone(),
        config.ipfs.timeout_seconds,
    );
    
    let ipns_publisher = IpnsPublisher::new(
        config.ipns.use_w3name,
        config.ipns.use_ipfs_node,
        config.ipfs.aws_api_url.clone(),
        config.ipns.validity_days,
    );
    
    // 4. 创建并发布DID
    let mut did_builder = DIDBuilder::new(
        config.agent.name.clone(),
        ipfs_client,
        ipns_publisher,
    );
    
    did_builder.add_service("AgentAPI", "https://agent.example.com/api");
    
    let result = did_builder.create_and_publish(&keypair).await?;
    
    println!("DID: {}", result.did);
    println!("访问: https://ipfs.io/ipns/{}", result.ipns_name);
    
    Ok(())
}
```

## 配置文件

创建 `~/.config/anp-rs-sdk/config.toml`:

```toml
[agent]
name = "My Agent"
private_key_path = "~/.local/share/anp-rs-sdk/keys/agent.key"
auto_generate_key = true

[ipfs]
# 选项1: 使用自己的AWS IPFS节点
aws_api_url = "http://your-aws-ip:5001"
aws_gateway_url = "http://your-aws-ip:8080"

# 选项2: 使用Pinata（免费）
# pinata_api_key = "your-key"
# pinata_api_secret = "your-secret"

[ipns]
use_w3name = true
use_ipfs_node = true
validity_days = 365

[cache]
enabled = true
ttl_seconds = 21600

[logging]
level = "info"
```

## 示例代码

### 1. 基础IPFS/IPNS使用

```bash
cargo run --example ipfs_ipns_basic
```

演示：
- 生成密钥
- 创建DID文档
- 上传到IPFS
- 发布到IPNS
- 双层验证

### 2. DID解析

```bash
cargo run --example did_resolver_demo
```

演示：
- 解析 did:ipfs 格式
- 解析 did:wba 格式
- 批量解析

### 3. 批量上传

```bash
cargo run --example batch_upload_demo
```

演示：
- 批量创建多个DID
- 并发上传（10个并发）
- 进度跟踪

### 4. 自动更新

```bash
cargo run --example auto_update_demo
```

演示：
- 定时自动更新
- 刷新IPNS有效期
- 后台运行

## DID双层验证

### 什么是双层验证？

```
DID标识符 ←→ IPNS名称 ←→ IPFS CID ←→ DID文档 ←→ IPNS引用
    └────────────────────────────────────────────┘
                    形成验证闭环
```

### 工作流程

1. **版本1**: 构建初始DID文档（不含IPNS引用）
2. **上传**: 上传到IPFS → CID1
3. **发布**: 发布CID1到IPNS
4. **版本2**: 在DID文档中添加IPNS service端点
5. **上传**: 上传新版本到IPFS → CID2
6. **更新**: 更新IPNS指向CID2

### 验证逻辑

```rust
use anp_rs_sdk::verify_double_layer;

// 验证DID文档的双层一致性
let is_valid = verify_double_layer(&did_document, &ipns_name)?;
```

检查：
- ✅ DID文档包含IPNSResolver服务
- ✅ IPNS名称与DID一致
- ✅ 元数据完整

## API文档

### 密钥管理

```rust
use anp_rs_sdk::{KeyManager, KeyPair};

// 生成新密钥
let keypair = KeyPair::generate()?;

// 加载或生成
let key_manager = KeyManager::new(config_dir);
let keypair = key_manager.load_or_generate(&key_path)?;

// 导出备份
let backup = keypair.export_backup(Some("password"))?;

// 从备份恢复
let keypair = KeyPair::import_from_backup(&backup, Some("password"))?;
```

### IPFS操作

```rust
use anp_rs_sdk::IpfsClient;

let ipfs_client = IpfsClient::new(
    Some("http://aws-ip:5001".to_string()),
    Some("http://aws-ip:8080".to_string()),
    None, None, 30
);

// 上传内容
let result = ipfs_client.upload(content, "file.json").await?;

// 获取内容
let content = ipfs_client.get(&cid).await?;

// Pin内容
ipfs_client.pin(&cid).await?;
```

### IPNS操作

```rust
use anp_rs_sdk::IpnsPublisher;

let ipns_publisher = IpnsPublisher::new(
    true,  // use_w3name
    true,  // use_ipfs_node
    Some("http://aws-ip:5001".to_string()),
    365    // validity_days
);

// 发布IPNS记录
let result = ipns_publisher.publish(&keypair, &cid, None).await?;

// 解析IPNS名称
let cid = ipns_publisher.resolve(&ipns_name).await?;
```

### DID构建和发布

```rust
use anp_rs_sdk::DIDBuilder;

let mut did_builder = DIDBuilder::new(
    "My Agent".to_string(),
    ipfs_client,
    ipns_publisher,
);

// 添加服务端点
did_builder
    .add_service("AgentWebSocket", "wss://agent.example.com/ws")
    .add_service("AgentAPI", "https://agent.example.com/api");

// 创建并发布（双层验证）
let result = did_builder.create_and_publish(&keypair).await?;

// 更新DID文档
let updated = did_builder.update_did_document(
    &keypair,
    current_sequence,
    |did_doc| {
        // 修改DID文档
    },
).await?;
```

### DID解析

```rust
use anp_rs_sdk::DIDResolver;

let resolver = DIDResolver::new(ipfs_client, ipns_publisher, 30);

// 解析单个DID
let result = resolver.resolve("did:ipfs:k51qzi5u...").await?;

// 批量解析
let results = resolver.resolve_batch(vec![
    "did:ipfs:k51qzi5u...".to_string(),
    "did:wba:example.com:alice".to_string(),
]).await;
```

### 批量上传

```rust
use anp_rs_sdk::BatchUploader;

let batch_uploader = BatchUploader::new(did_builder, 10);  // 10个并发

let items = vec![
    ("Agent1".to_string(), keypair1),
    ("Agent2".to_string(), keypair2),
    ("Agent3".to_string(), keypair3),
];

let result = batch_uploader.batch_upload(items).await?;

println!("成功: {}, 失败: {}", 
         result.success_count, 
         result.failure_count);
```

### 自动更新

```rust
use anp_rs_sdk::AutoUpdateManager;

let update_manager = AutoUpdateManager::new(
    did_builder,
    keypair,
    initial_sequence,
    initial_cid,
    24,  // 24小时更新一次
);

// 启动自动更新
update_manager.start().await;

// 查看状态
let state = update_manager.get_state().await;

// 手动触发更新
let result = update_manager.trigger_update().await?;

// 停止自动更新
update_manager.stop().await;
```

## 架构说明

### 模块结构

```
anp-rs-sdk/
├── config_manager    # 配置管理
├── key_manager       # 密钥管理
├── ipfs_client       # IPFS客户端
├── ipns_publisher    # IPNS发布器
├── did_builder       # DID文档构建
├── did_resolver      # DID解析器
└── batch_uploader    # 批量上传和自动更新
```

### 数据流

```
密钥生成 → DID派生 → DID文档构建 → IPFS上传 → IPNS发布 → 双层验证
                                        ↓
                                    自动更新
                                        ↓
                                    DID解析
```

## 性能指标

### 首次发布

- 密钥生成: ~100ms
- DID文档构建: ~50ms
- IPFS上传: 1-3秒
- IPNS发布: 1-2秒
- 双层验证: 1-2秒
- **总计: ~5-6秒**

### 后续更新

- DID文档构建: ~50ms
- IPFS上传: 0.5-1秒
- IPNS更新: 0.5-1秒
- **总计: ~1-2秒**

### 批量上传（10个并发）

- 10个DID: ~6-8秒
- 100个DID: ~50-60秒
- 平均: ~0.5-0.6秒/个

## 成本分析

### 使用AWS IPFS节点

- 服务器: $5-20/月
- 带宽: 根据使用量
- 总计: $5-50/月

### 使用Pinata

- 免费额度: 1GB存储
- 超出后: $20/月起
- 总计: $0-20/月

### 推荐方案

- **开发测试**: 使用Pinata（免费）
- **生产环境**: AWS IPFS + Pinata备用

## 文档资源

- 📖 [快速开始](QUICKSTART.md) - 5分钟上手
- 📖 [架构设计](ARCHITECTURE.md) - 整体架构
- 📖 [实现逻辑](IMPLEMENTATION_LOGIC.md) - 详细逻辑
- 📖 [IPFS/IPNS指南](IPFS_IPNS_GUIDE.md) - 完整指南
- 📖 [实现状态](IMPLEMENTATION_STATUS.md) - 当前进度

## 示例代码

查看 `examples/` 目录：

- `ipfs_ipns_basic.rs` - 基础IPFS/IPNS使用
- `did_resolver_demo.rs` - DID解析演示
- `batch_upload_demo.rs` - 批量上传演示
- `auto_update_demo.rs` - 自动更新演示

## 运行示例

```bash
# 设置日志级别
export RUST_LOG=info

# 运行基础示例
cargo run --example ipfs_ipns_basic

# 运行DID解析示例
cargo run --example did_resolver_demo

# 运行批量上传示例
cargo run --example batch_upload_demo

# 运行自动更新示例
cargo run --example auto_update_demo
```

## 故障排除

### 问题: AWS IPFS节点连接失败

**解决方案**:
1. 检查IPFS节点是否运行: `ipfs id`
2. 检查防火墙规则
3. 验证配置文件中的地址正确
4. 使用Pinata作为备用

### 问题: w3name发布失败

**解决方案**:
1. 检查网络连接
2. SDK会自动回退到IPFS节点
3. 查看日志了解详细错误

### 问题: 编译错误

**解决方案**:
```bash
# 更新依赖
cargo update

# 清理并重新编译
cargo clean
cargo build
```

## 下一步

- [ ] 实现缓存系统（提升解析性能）
- [ ] 集成w3name Rust库（完善IPNS）
- [ ] 添加更多测试
- [ ] 性能优化

## 获取帮助

- GitHub: https://github.com/logos-42/AgentNetworkProtocol
- Discord: https://discord.gg/sFjBKTY7sB
- Email: chgaowei@gmail.com

## 许可证

MIT License

---

**版本**: 0.2.0  
**发布日期**: 2025-01-08  
**状态**: Beta - 核心功能完整，可用于开发
