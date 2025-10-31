# IPNS 自动发布功能使用指南

## 概述

本指南介绍如何使用 DIAP Rust SDK 的 IPNS（InterPlanetary Name System）自动发布功能。IPNS 允许您创建可变的、人类可读的指针，指向 IPFS 内容（CID），而无需每次内容更新时都更改引用。

## 功能特性

- ✅ **自动 Key 管理**: 首次使用时自动创建 IPNS key，后续运行自动复用
- ✅ **离线发布**: 使用 `allow-offline=true` 参数，无需等待网络传播
- ✅ **长期有效**: 支持自定义 lifetime（默认 1 年）
- ✅ **本地优先**: 仅使用 127.0.0.1，不泄露本机对外 IP
- ✅ **错误容错**: IPNS 发布失败不影响主流程继续执行
- ✅ **验证机制**: 自动验证 IPNS 和 IPFS 网关可访问性

## 前置要求

1. **本地 Kubo IPFS 节点**
   ```bash
   # 安装 Kubo (go-ipfs)
   # 参考: https://docs.ipfs.tech/install/command-line/
   
   # 启动 IPFS 守护进程
   ipfs daemon
   ```

2. **默认端口配置**
   - API: `http://127.0.0.1:5001`
   - 网关: `http://127.0.0.1:8081`

## API 使用

### 1. 基础用法

```rust
use diap_rs_sdk::IpfsClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建 IPFS 客户端
    let ipfs_client = IpfsClient::new_with_remote_node(
        "http://127.0.0.1:5001".to_string(),
        "http://127.0.0.1:8081".to_string(),
        30  // 超时时间（秒）
    );
    
    // 上传内容到 IPFS
    let content = r#"{"message": "Hello IPNS!"}"#;
    let upload_result = ipfs_client.upload(content, "test.json").await?;
    println!("CID: {}", upload_result.cid);
    
    // 发布到 IPNS（一步完成）
    let ipns_result = ipfs_client.publish_after_upload(
        &upload_result.cid,
        "my_key",      // IPNS key 名称
        "8760h",       // lifetime: 1 年
        "1h"           // TTL: 1 小时
    ).await?;
    
    println!("IPNS 名称: /ipns/{}", ipns_result.name);
    println!("IPNS 值: {}", ipns_result.value);
    println!("网关访问: http://127.0.0.1:8081/ipns/{}", ipns_result.name);
    
    Ok(())
}
```

### 2. 分步操作

```rust
// 步骤 1: 确保 key 存在
let key_name = "diap";
let key = ipfs_client.ensure_key_exists(key_name).await?;
println!("Key '{}' 已准备好", key);

// 步骤 2: 发布 IPNS 记录
let ipns_result = ipfs_client.publish_ipns(
    &cid,
    key_name,
    "8760h",  // lifetime
    "1h"      // TTL
).await?;

println!("发布成功: /ipns/{}", ipns_result.name);
```

### 3. 更新 IPNS 记录

```rust
// 上传新内容
let new_content = r#"{"message": "Updated content!"}"#;
let new_upload = ipfs_client.upload(new_content, "updated.json").await?;

// 使用相同的 key 更新 IPNS 记录
let updated_ipns = ipfs_client.publish_after_upload(
    &new_upload.cid,
    "my_key",  // 使用相同的 key 名称
    "8760h",
    "1h"
).await?;

// IPNS 名称保持不变，但现在指向新的 CID
println!("IPNS 已更新: /ipns/{} -> {}", updated_ipns.name, updated_ipns.value);
```

## 命令行示例

### 运行完整闭环示例（启用 IPNS）

```bash
# 基本用法
cargo run --example iroh_complete_closed_loop -- \
  --enable-ipns

# 自定义配置
cargo run --example iroh_complete_closed_loop -- \
  --api-url http://127.0.0.1:5001 \
  --gateway-url http://127.0.0.1:8081 \
  --enable-ipns \
  --ipns-key diap \
  --ipns-lifetime 8760h \
  --ipns-ttl 1h

# 使用环境变量
export DIAP_IPFS_API_URL=http://127.0.0.1:5001
export DIAP_IPFS_GATEWAY_URL=http://127.0.0.1:8081

cargo run --example iroh_complete_closed_loop -- --enable-ipns
```

### 运行 IPNS 测试示例

```bash
# 测试 IPNS 发布功能
cargo run --example test_ipns_publish

# 使用自定义 IPFS 节点
DIAP_IPFS_API_URL=http://localhost:5001 \
DIAP_IPFS_GATEWAY_URL=http://localhost:8080 \
cargo run --example test_ipns_publish
```

## 参数说明

### IpfsClient 方法

#### `ensure_key_exists(key_name: &str) -> Result<String>`
- **功能**: 确保指定名称的 IPNS key 存在
- **参数**:
  - `key_name`: key 的名称（如 "diap", "my_app"）
- **返回**: key 名称（与输入相同）
- **行为**: 
  - 如果 key 不存在，自动创建（类型: ed25519）
  - 如果 key 已存在，直接返回

#### `publish_ipns(cid: &str, key_name: &str, lifetime: &str, ttl: &str) -> Result<IpnsPublishResult>`
- **功能**: 发布 IPNS 记录
- **参数**:
  - `cid`: 要发布的 IPFS CID
  - `key_name`: 使用的 IPNS key 名称
  - `lifetime`: 记录的生命周期（如 "24h", "8760h"）
  - `ttl`: 缓存时间（如 "1h", "30m"）
- **返回**: `IpnsPublishResult` 结构体
  - `name`: IPNS 名称（PeerID）
  - `value`: IPNS 值（/ipfs/<CID> 路径）
  - `published_at`: 发布时间戳

#### `publish_after_upload(cid: &str, key_name: &str, lifetime: &str, ttl: &str) -> Result<IpnsPublishResult>`
- **功能**: 便捷方法，自动确保 key 存在后发布
- **参数**: 同 `publish_ipns`
- **返回**: 同 `publish_ipns`
- **推荐**: 大多数情况下使用此方法

### 时间格式

支持以下时间单位：
- `s`: 秒（如 "30s"）
- `m`: 分钟（如 "30m"）
- `h`: 小时（如 "24h", "8760h"）

常用配置：
- **短期测试**: lifetime="1h", ttl="5m"
- **日常使用**: lifetime="168h" (7天), ttl="1h"
- **长期发布**: lifetime="8760h" (1年), ttl="1h"

## 输出示例

### 成功发布

```
📣 发布 IPNS 记录 (key=diap)...
   🔑 确保 IPNS key 'diap' 存在...
   ✅ IPNS key 'diap' 已准备好
   📤 发布 Alice 的 IPNS 记录...
   ✅ Alice IPNS: /ipns/QmXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX -> /ipfs/QmYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
   📤 发布 Bob 的 IPNS 记录...
   ✅ Bob   IPNS: /ipns/QmZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ -> /ipfs/QmAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
   🌐 网关访问: http://127.0.0.1:8081/ipns/QmXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
   ✅ IPNS 与 IPFS 网关均可访问
   ✅ IPNS 发布完成
```

### 错误处理

```
   ❌ IPNS key 创建/检查失败: 连接被拒绝 (os error 10061) (继续执行)
   提示: 请确保本地 Kubo IPFS 节点正在运行
```

## 常见问题

### Q1: IPNS 发布失败，提示连接被拒绝

**A**: 确保本地 IPFS 守护进程正在运行：
```bash
# 检查 IPFS 是否运行
ipfs id

# 如果未运行，启动守护进程
ipfs daemon
```

### Q2: IPNS 网关访问返回 404

**A**: IPNS 记录需要时间传播到网络。本地网关通常最快（几秒内），公共网关可能需要几分钟。

### Q3: 如何查看已创建的 IPNS keys？

**A**: 使用 IPFS CLI：
```bash
ipfs key list -l
```

### Q4: 可以删除不需要的 IPNS key 吗？

**A**: 可以，使用 IPFS CLI：
```bash
ipfs key rm <key_name>
```

### Q5: IPNS 记录会过期吗？

**A**: 会的，根据设置的 `lifetime` 参数。过期后需要重新发布。建议设置较长的 lifetime（如 1 年）。

### Q6: 同一个 key 可以发布多次吗？

**A**: 可以！每次发布会更新该 key 指向的 CID。这正是 IPNS 的核心功能。

## 最佳实践

1. **Key 命名**: 使用有意义的名称，如 "app_name", "agent_did"
2. **Lifetime 设置**: 
   - 开发测试: 1-24 小时
   - 生产环境: 7-30 天或更长
3. **TTL 设置**: 通常设置为 1 小时，平衡更新速度和缓存效率
4. **错误处理**: 始终处理 IPNS 发布失败的情况，不要让它阻塞主流程
5. **本地优先**: 优先使用本地 IPFS 节点，避免依赖公共服务

## 安全注意事项

1. **私钥保护**: IPNS key 的私钥存储在 IPFS 数据目录中，确保目录权限正确
2. **网络隔离**: 默认配置仅使用 127.0.0.1，不会暴露到公网
3. **内容验证**: IPNS 记录是签名的，可以验证发布者身份
4. **访问控制**: 考虑在生产环境中限制 IPFS API 的访问权限

## 与 DID/ZKP 集成

IPNS 发布功能与现有的 DID/ZKP/CID 闭环完全兼容：

```rust
// 1. 注册 DID（获得 CID）
let registration = auth_mgr.register_agent(&agent_info, &keypair, &peer_id).await?;

// 2. 发布到 IPNS（可选）
let ipns_result = ipfs_client.publish_after_upload(
    &registration.cid,
    "agent_did",
    "8760h",
    "1h"
).await?;

// 3. 使用 IPNS 名称作为稳定的引用
println!("Agent DID 可通过 IPNS 访问: /ipns/{}", ipns_result.name);
```

## 技术细节

### IPFS API 调用

1. **Key 列表**: `POST /api/v0/key/list`
2. **Key 生成**: `POST /api/v0/key/gen?arg=<name>&type=ed25519`
3. **IPNS 发布**: `POST /api/v0/name/publish?arg=/ipfs/<cid>&key=<key>&allow-offline=true&resolve=true&lifetime=<lifetime>&ttl=<ttl>`

### 参数说明

- `allow-offline=true`: 允许离线发布，不等待网络传播
- `resolve=true`: 发布前解析路径，确保 CID 有效
- `type=ed25519`: 使用 Ed25519 签名算法（性能最佳）

## 相关资源

- [IPFS 官方文档](https://docs.ipfs.tech/)
- [IPNS 概念介绍](https://docs.ipfs.tech/concepts/ipns/)
- [Kubo (go-ipfs) 安装指南](https://docs.ipfs.tech/install/command-line/)
- [DIAP Rust SDK 文档](./README.md)

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
