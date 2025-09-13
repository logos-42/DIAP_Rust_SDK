# 发布 ANP Rust SDK 到 Crates.io 指南

## 📋 发布前检查清单

### ✅ 1. 项目配置检查
- [x] `Cargo.toml` 包含所有必要字段
- [x] `README.md` 文件存在且内容完整
- [x] `LICENSE` 文件存在
- [x] 所有示例都能正常运行
- [x] 代码编译无错误

### ✅ 2. 包元数据
- [x] 包名：`anp-rs-sdk`
- [x] 版本：`1.0.0`
- [x] 描述：完整的功能描述
- [x] 关键词：相关技术关键词
- [x] 分类：正确的包分类
- [x] 许可证：MIT
- [x] 仓库链接：GitHub 仓库
- [x] 文档链接：docs.rs 链接

## 🚀 发布步骤

### 步骤 1：注册 Crates.io 账户
1. 访问 [crates.io](https://crates.io)
2. 使用 GitHub 账户登录
3. 验证邮箱地址

### 步骤 2：获取 API 令牌
1. 登录后访问 [账户设置](https://crates.io/me)
2. 点击 "New Token"
3. 输入令牌名称（如：`anp-rs-sdk-publish`）
4. 复制生成的令牌

### 步骤 3：配置 Cargo
```bash
cargo login <your-api-token>
```

### 步骤 4：验证包
```bash
# 检查包内容
cargo package --list

# 验证包（不实际创建）
cargo package --allow-dirty
```

### 步骤 5：发布包
```bash
# 发布到 crates.io
cargo publish
```

## 📦 发布后操作

### 1. 验证发布
- 访问 [crates.io/crates/anp-rs-sdk](https://crates.io/crates/anp-rs-sdk)
- 确认包信息正确显示
- 测试安装：`cargo install anp-rs-sdk`

### 2. 更新文档
- 文档会自动发布到 [docs.rs/anp-rs-sdk](https://docs.rs/anp-rs-sdk)
- 等待文档构建完成（通常需要几分钟）

### 3. 版本管理
- 使用语义化版本控制
- 重大更改：主版本号 +1
- 新功能：次版本号 +1
- 错误修复：修订版本号 +1

## 🔄 更新版本

### 更新版本号
```bash
# 编辑 Cargo.toml 中的 version 字段
# 例如：从 "1.0.0" 改为 "1.0.1"
```

### 发布新版本
```bash
cargo publish
```

## 📊 包统计

发布后，你可以在 crates.io 上看到：
- 下载统计
- 依赖关系
- 版本历史
- 用户反馈

## 🛠️ 故障排除

### 常见问题

1. **包名已存在**
   - 解决方案：更改包名或联系现有维护者

2. **API 令牌无效**
   - 解决方案：重新生成令牌并重新登录

3. **文档构建失败**
   - 解决方案：检查代码中的文档注释

4. **依赖版本冲突**
   - 解决方案：更新依赖版本或调整版本约束

## 📚 相关资源

- [Crates.io 用户指南](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [语义化版本控制](https://semver.org/)
- [Rust 文档指南](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
