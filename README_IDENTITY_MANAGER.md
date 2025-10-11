# 统一身份管理模块 (IdentityManager)

## 📖 概述

`IdentityManager` 是 DIAP Rust SDK 的统一身份管理模块，提供了简化的 DID/IPNS 注册和验证接口。

### 🎯 核心优势

- **一键注册** - `register_identity()` 自动完成 DID 文档生成、IPFS 上传、IPNS 绑定
- **一键验证** - `verify_identity()` 自动完成 IPNS 解析、文档下载、签名验证
- **自动绑定** - DID ↔ IPNS ↔ CID 自动关联
- **双层验证** - 自动验证 DID 文档与 IPNS 的一致性
- **简化 API** - 用户只需关心 `AgentInfo` 和 `KeyPair`

## 🚀 快速开始

### 1. 初始化身份管理器

```rust
use diap_rs_sdk::*;

// 配置 IPFS 客户端
let ipfs_client = IpfsClient::new(
    Some("http://localhost:5001".to_string()),  // IPFS API URL
    Some("http://localhost:8080".to_string()),  // IPFS Gateway URL
    None,                                        // Pinata API key (可选)
    None,                                        // Pinata API secret (可选)
    30,                                          // 超时时间（秒）
);

// 配置 IPNS 发布器
let ipns_publisher = IpnsPublisher::new(
    true,  // 使用 w3name
    true,  // 使用 IPFS 节点
    Some("http://localhost:5001".to_string()),
    365,   // IPNS 记录有效期（天）
);

// 创建身份管理器
let identity_manager = IdentityManager::new(ipfs_client, ipns_publisher);
```

### 2. 生成密钥对

```rust
// 生成新密钥对
let keypair = KeyPair::generate()?;

// 或从文件加载
let key_path = std::path::PathBuf::from("./my_identity.key");
let keypair = KeyPair::from_file(&key_path)?;

// 保存密钥到文件
keypair.save_to_file(&key_path)?;
```

### 3. 准备智能体信息

```rust
let agent_info = AgentInfo {
    name: "我的智能体".to_string(),
    services: vec![
        ServiceInfo {
            service_type: "API".to_string(),
            endpoint: "https://api.myagent.com".to_string(),
        },
        ServiceInfo {
            service_type: "WebSocket".to_string(),
            endpoint: "wss://ws.myagent.com".to_string(),
        },
    ],
    description: Some("一个强大的去中心化智能体".to_string()),
    tags: Some(vec!["AI".to_string(), "DeFi".to_string()]),
};
```

### 4. 一键注册身份

```rust
let registration = identity_manager
    .register_identity(&agent_info, &keypair)
    .await?;

println!("✅ 注册成功！");
println!("  DID: {}", registration.did);
println!("  IPNS: {}", registration.ipns_name);
println!("  CID: {}", registration.cid);
```

**内部自动完成的步骤：**
1. 构建 DID 文档（版本 1，不含 IPNS 引用）
2. 上传到 IPFS，获取 CID1
3. 发布 CID1 到 IPNS，获取 IPNS name
4. 在 DID 文档中添加 IPNS service 端点
5. 构建 DID 文档（版本 2，含 IPNS 引用）
6. 上传到 IPFS，获取 CID2
7. 更新 IPNS 指向 CID2

### 5. 一键验证身份

```rust
// 通过 IPNS name 验证
let verification = identity_manager
    .verify_identity(&registration.ipns_name)
    .await?;

if verification.is_valid {
    println!("✅ 身份验证成功");
    println!("智能体名称: {}", verification.agent_info.name);
    println!("服务数量: {}", verification.agent_info.services.len());
} else {
    println!("❌ 身份验证失败");
}

// 查看验证详情
for detail in &verification.verification_details {
    println!("{}", detail);
}
```

**内部自动完成的步骤：**
1. 通过 IPNS name 解析到最新 DID 文档 CID
2. 从 IPFS 下载 DID 文档
3. 解析 DID 文档
4. 验证双层一致性（DID ↔ IPNS 绑定）
5. 验证 DID 与 IPNS name 的匹配性
6. 提取智能体信息

### 6. 通过 DID 直接验证

```rust
let verification = identity_manager
    .resolve_by_did(&registration.did)
    .await?;

println!("智能体信息: {:?}", verification.agent_info);
```

### 7. 更新身份信息

```rust
// 更新智能体信息
let mut updated_agent_info = agent_info.clone();
updated_agent_info.services.push(ServiceInfo {
    service_type: "GraphQL".to_string(),
    endpoint: "https://graphql.myagent.com".to_string(),
});

// 获取当前序列号
let current_sequence = registration.did_document
    .ipfs_metadata
    .as_ref()
    .map(|m| m.sequence)
    .unwrap_or(1);

// 更新身份
let updated_registration = identity_manager
    .update_identity(&updated_agent_info, &keypair, current_sequence)
    .await?;

println!("✅ 身份更新成功");
println!("  新 CID: {}", updated_registration.cid);
```

## 📋 API 参考

### IdentityManager

#### `new(ipfs_client, ipns_publisher) -> Self`
创建新的身份管理器。

#### `register_identity(agent_info, keypair) -> Result<IdentityRegistration>`
一键注册身份（DID + IPFS + IPNS）。

**参数：**
- `agent_info: &AgentInfo` - 智能体信息
- `keypair: &KeyPair` - 密钥对

**返回：**
- `IdentityRegistration` - 注册结果，包含 DID、IPNS name、CID 和 DID 文档

#### `verify_identity(ipns_name) -> Result<IdentityVerification>`
一键验证身份（通过 IPNS name）。

**参数：**
- `ipns_name: &str` - IPNS 名称

**返回：**
- `IdentityVerification` - 验证结果，包含智能体信息和验证状态

#### `resolve_by_did(did) -> Result<IdentityVerification>`
通过 DID 直接验证身份。

**参数：**
- `did: &str` - DID 标识符

**返回：**
- `IdentityVerification` - 验证结果

#### `update_identity(agent_info, keypair, current_sequence) -> Result<IdentityRegistration>`
更新身份信息。

**参数：**
- `agent_info: &AgentInfo` - 更新后的智能体信息
- `keypair: &KeyPair` - 密钥对
- `current_sequence: u64` - 当前 IPNS 序列号

**返回：**
- `IdentityRegistration` - 更新后的注册结果

### 数据结构

#### AgentInfo
```rust
pub struct AgentInfo {
    pub name: String,                    // 智能体名称
    pub services: Vec<ServiceInfo>,      // 服务端点列表
    pub description: Option<String>,     // 描述信息
    pub tags: Option<Vec<String>>,       // 标签
}
```

#### ServiceInfo
```rust
pub struct ServiceInfo {
    pub service_type: String,    // 服务类型（如 "API", "WebSocket"）
    pub endpoint: String,        // 服务端点 URL
}
```

#### IdentityRegistration
```rust
pub struct IdentityRegistration {
    pub did: String,                     // DID 标识符
    pub ipns_name: String,               // IPNS 名称
    pub cid: String,                     // 当前 CID
    pub did_document: DIDDocument,       // DID 文档
    pub registered_at: String,           // 注册时间
}
```

#### IdentityVerification
```rust
pub struct IdentityVerification {
    pub did: String,                           // DID 标识符
    pub agent_info: AgentInfo,                 // 智能体信息
    pub is_valid: bool,                        // 验证状态
    pub verification_details: Vec<String>,     // 验证详情
    pub verified_at: String,                   // 验证时间
}
```

## 🎬 运行演示

```bash
# 确保 IPFS 节点正在运行
ipfs daemon

# 运行统一身份管理演示
cargo run --example unified_identity_demo
```

## 🔄 与原有模块的关系

`IdentityManager` 是对以下模块的高层封装：

- **DIDBuilder** - DID 文档构建和发布
- **DIDResolver** - DID 解析和验证
- **IpfsClient** - IPFS 上传/下载
- **IpnsPublisher** - IPNS 发布/解析
- **KeyPair** - 密钥管理

**优势：**
- ✅ 保留原有模块，不破坏现有架构
- ✅ 提供更简洁的高层 API
- ✅ 向下兼容，仍可直接使用底层模块
- ✅ 自动处理复杂的双层验证流程

## 📝 完整示例

查看 `examples/unified_identity_demo.rs` 获取完整的演示代码。

## ⚠️ 注意事项

1. **IPFS 节点** - 需要本地或远程 IPFS 节点正在运行
2. **网络连接** - 需要网络连接以访问 IPFS/IPNS 服务
3. **密钥安全** - 妥善保管密钥文件，不要泄露私钥
4. **序列号** - 更新身份时需要提供正确的序列号

## 🔗 相关文档

- [DID 规范](./README.md#did-标识符)
- [IPFS/IPNS 集成](./README_IPFS_IPNS.md)
- [双层验证流程](./LIBP2P_INTEGRATION_SUMMARY.md)
- [更新日志](./CHANGELOG.md)

