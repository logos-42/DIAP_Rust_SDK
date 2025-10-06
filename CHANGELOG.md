# 更新日志

## [0.1.2] - 2025-10-06

### ✨ 新增功能

#### 1. **真实的 HTTP 路由输出**
- ✅ 修复了 DID 文档路由，`GET /.well-known/did.json` 现在返回真实的 DID 文档
- ✅ 修复了 AD 文档路由，`GET /agents/{id}/ad.json` 现在返回真实的智能体描述文档
- ✅ 实现了 ANP API 端点，`POST /anp/api` 可以接收和处理 ANP 协议消息
- ✅ 添加了适当的错误处理和 404 响应

#### 2. **双 DID 格式支持**
- ✅ 同时支持 `did:wba` 和 `did:web` 格式
- ✅ `KeyPairResult` 现在包含 `did_web` 字段
- ✅ `DIDConfig` 和 `AgentConfig` 都包含 `did_web` 字段
- ✅ 自动生成符合 W3C 标准的 `did:web` 标识符

**示例**:
```rust
let config = sdk.start().await?;
println!("DID (wba): {}", config.did);
// 输出: did:wba:127.0.0.1:3000:auto-agent

println!("DID (web): {}", config.did_web.unwrap());
// 输出: did:web:127.0.0.1%3A3000:auto-agent
```

#### 3. **IPFS 注册表集成**
- ✅ 新增 `ipfs_registry` 模块，提供完整的 IPFS 注册表功能
- ✅ 支持将智能体信息发布到 IPFS 网络
- ✅ 支持从 IPFS 查询智能体信息
- ✅ 支持发布和查询注册表索引（多个智能体的列表）
- ✅ 支持按能力、接口等条件搜索智能体
- ✅ 可配置的 IPFS API 和网关地址
- ✅ 支持内容固定（pin）到本地节点

**使用方式**:
```rust
// 自动注册到 IPFS
let options = AutoConfigOptions {
    auto_ipfs_register: Some(true),
    ipfs_config: Some(IpfsRegistryConfig {
        api_url: "http://127.0.0.1:5001".to_string(),
        gateway_url: "https://ipfs.io".to_string(),
        pin: true,
    }),
    ..Default::default()
};

let config = sdk.start().await?;
if let Some(cid) = config.ipfs_cid {
    println!("IPFS CID: {}", cid);
    println!("访问: https://ipfs.io/ipfs/{}", cid);
}
```

**手动使用**:
```rust
use anp_rs_sdk::{IpfsRegistry, IpfsRegistryConfig, AgentRegistryEntry};

let registry = IpfsRegistry::new(IpfsRegistryConfig::default());

// 发布智能体
let cid = registry.publish_agent(entry).await?;

// 查询智能体
let agent = registry.query_agent(&cid).await?;

// 搜索智能体
let results = registry.search_agents(&index_cid, filter).await?;
```

### 📚 新增示例

1. **basic_agent_with_did_web.rs**
   - 展示双 DID 格式支持
   - 演示真实的 HTTP 端点
   - 自动测试所有端点
   - 适合快速入门

2. **complete_agent_with_ipfs.rs**
   - 完整功能演示
   - 包含 IPFS 注册
   - 展示端到端工作流
   - 适合了解全部功能

3. **ipfs_registry_demo.rs**
   - 专注于 IPFS 功能
   - 演示发布、查询、搜索
   - 包含故障排除提示
   - 适合 IPFS 集成开发

### 🔧 API 变更

#### 新增类型

```rust
// IPFS 注册表配置
pub struct IpfsRegistryConfig {
    pub api_url: String,
    pub gateway_url: String,
    pub pin: bool,
}

// 智能体注册信息
pub struct AgentRegistryEntry {
    pub did: String,
    pub did_web: Option<String>,
    pub name: String,
    pub endpoint: String,
    pub did_document_url: String,
    pub ad_url: String,
    pub capabilities: Vec<String>,
    pub interfaces: Vec<String>,
    pub registered_at: String,
    pub updated_at: String,
}

// IPFS 注册表
pub struct IpfsRegistry { /* ... */ }

// 搜索过滤器
pub struct AgentSearchFilter {
    pub did: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub interfaces: Option<Vec<String>>,
}
```

#### 修改的类型

```rust
// AutoConfigOptions 新增字段
pub struct AutoConfigOptions {
    // ... 原有字段 ...
    pub auto_ipfs_register: Option<bool>,  // 新增
    pub ipfs_config: Option<IpfsRegistryConfig>,  // 新增
}

// AgentConfig 新增字段
pub struct AgentConfig {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
    pub ipfs_cid: Option<String>,  // 新增
}

// KeyPairResult 新增字段
pub struct KeyPairResult {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
}

// DIDConfig 新增字段
pub struct DIDConfig {
    // ... 原有字段 ...
    pub did_web: Option<String>,  // 新增
}
```

#### 新增方法

```rust
// HTTPAutoConfig
impl HTTPAutoConfig {
    pub async fn set_did_document(&self, doc: Value);
    pub async fn set_ad_document(&self, doc: Value);
}

// IpfsRegistry
impl IpfsRegistry {
    pub fn new(config: IpfsRegistryConfig) -> Self;
    pub async fn publish_agent(&self, entry: AgentRegistryEntry) -> Result<String>;
    pub async fn query_agent(&self, cid: &str) -> Result<AgentRegistryEntry>;
    pub async fn publish_registry_index(&self, entries: Vec<AgentRegistryEntry>) -> Result<String>;
    pub async fn query_registry_index(&self, cid: &str) -> Result<RegistryIndex>;
    pub async fn search_agents(&self, index_cid: &str, filter: AgentSearchFilter) -> Result<Vec<AgentRegistryEntry>>;
}
```

### 📦 依赖更新

- `reqwest`: 添加 `multipart` feature
- `multipart`: 新增依赖 0.18

### 🐛 修复

- 修复了 HTTP 路由只返回占位符的问题
- 修复了 DID 文档和 AD 文档无法访问的问题
- 改进了错误处理和用户反馈

### 📖 文档改进

- 更新了 README.md，添加新功能说明
- 添加了详细的使用示例
- 添加了 IPFS 集成指南
- 改进了 API 文档注释

### ⚠️ 注意事项

1. **IPFS 功能**：
   - 需要本地运行 IPFS 节点（`ipfs daemon`）
   - 默认 API 端口: 5001
   - 默认网关端口: 8080
   - 可以使用公共 IPFS 网关查看内容

2. **DID Web 格式**：
   - `did:web` 格式中的端口号使用 `%3A` 编码
   - 示例: `did:web:example.com%3A3000:agent`
   - 解析时需要将 `:` 替换回端口号

3. **向后兼容**：
   - 所有新增字段都是 `Option<T>` 类型
   - 现有代码无需修改即可升级
   - IPFS 功能默认关闭

### 🚀 使用建议

#### 快速开始（不使用 IPFS）
```bash
cargo run --example basic_agent_with_did_web
```

#### 完整功能（包含 IPFS）
```bash
# 1. 启动 IPFS 节点
ipfs daemon

# 2. 运行示例
cargo run --example complete_agent_with_ipfs
```

#### 集成到现有项目
```rust
use anp_rs_sdk::{ANPSDK, AutoConfigOptions};

let options = AutoConfigOptions {
    auto_ipfs_register: Some(false),  // 暂不使用 IPFS
    ..Default::default()
};

let mut sdk = ANPSDK::new(options);
let config = sdk.start().await?;

// 访问 DID 文档
// GET http://127.0.0.1:{port}/.well-known/did.json

// 访问 AD 文档
// GET http://127.0.0.1:{port}/agents/auto-agent/ad.json
```

---

## [0.1.1] - 2025-10-06

### 🐛 修复
- 修复了编译警告
- 清理了未使用的代码

---

## [0.1.0] - 2025-10-06

### 🎉 初始版本
- HTTP 服务器自动配置
- DID 自动生成（Ed25519、secp256k1）
- 智能体描述文档生成
- 基础示例和文档

