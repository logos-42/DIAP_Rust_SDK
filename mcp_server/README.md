# DIAP MCP Server

基于 Rust SDK 的 Model Context Protocol (MCP) 服务器，提供去中心化智能体身份管理和 IPFS 存储服务。

## 架构

```
Python MCP Server (mcp 协议)
    ↓
PyO3 绑定层
    ↓
Rust DIAP SDK (核心功能)
    ↓
IPFS 网络
```

## 为什么使用 Python + Rust？

1. **MCP 协议支持**: MCP 官方 SDK 主要是 Python 和 TypeScript 实现
2. **性能优势**: 核心功能用 Rust 实现，保证高性能和安全性
3. **易于集成**: Python 层提供简单的 MCP 接口，易于与 AI 工具集成
4. **云部署友好**: Python 服务器易于部署到各种云平台

## 安装

### 1. 构建 Rust 扩展

```bash
cd mcp_server
pip install maturin
maturin develop
```

### 2. 安装 Python 依赖

```bash
pip install -e .
```

## 使用

### 启动服务器

```bash
python -m diap_mcp_server.server
```

### 配置 MCP 客户端

在你的 MCP 客户端配置中添加：

```json
{
  "mcpServers": {
    "diap": {
      "command": "python",
      "args": ["-m", "diap_mcp_server.server"],
      "env": {}
    }
  }
}
```

## 可用工具

### 1. create_agent
创建新的去中心化智能体

```json
{
  "name": "create_agent",
  "arguments": {
    "name": "MyAgent",
    "ipfs_api": "http://localhost:5001"
  }
}
```

### 2. upload_to_ipfs
上传内容到 IPFS

```json
{
  "name": "upload_to_ipfs",
  "arguments": {
    "content": "{\"data\": \"example\"}"
  }
}
```

### 3. get_from_ipfs
从 IPFS 获取内容

```json
{
  "name": "get_from_ipfs",
  "arguments": {
    "cid": "QmXxx..."
  }
}
```

### 4. verify_agent
验证智能体身份（零知识证明）

```json
{
  "name": "verify_agent",
  "arguments": {
    "did": "did:diap:...",
    "proof": "..."
  }
}
```

## 云部署

### Docker 部署

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cd mcp_server && cargo build --release

FROM python:3.11-slim
WORKDIR /app
COPY --from=builder /app/mcp_server/target/release/libdiap_mcp_server.so .
COPY mcp_server/python ./python
RUN pip install mcp httpx pydantic
CMD ["python", "-m", "diap_mcp_server.server"]
```

### 云平台部署

支持部署到：
- AWS Lambda (使用 AWS Lambda Python Runtime)
- Google Cloud Run
- Azure Functions
- 任何支持 Python 的云平台

## 开发

### 运行测试

```bash
pytest tests/
```

### 重新构建

```bash
maturin develop --release
```

## 许可证

MIT
