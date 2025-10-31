FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# 构建 Rust 库
WORKDIR /app/mcp_server
RUN cargo build --release

# Python 运行时
FROM python:3.11-slim

WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 复制 Rust 编译产物
COPY --from=builder /app/mcp_server/target/release/libdiap_mcp_server.so /app/

# 复制 Python 代码
COPY mcp_server/python /app/python
COPY mcp_server/pyproject.toml /app/

# 安装 Python 依赖
RUN pip install --no-cache-dir mcp httpx pydantic

# 设置环境变量
ENV PYTHONPATH=/app/python
ENV RUST_LOG=info

# 暴露端口（如果需要 HTTP 接口）
EXPOSE 8080

# 启动服务
CMD ["python", "-m", "diap_mcp_server.server"]
