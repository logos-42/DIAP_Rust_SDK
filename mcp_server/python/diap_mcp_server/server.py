"""DIAP MCP Server Implementation"""

import asyncio
from typing import Any
from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent
from pydantic import AnyUrl
import logging

# 导入 Rust 绑定
from ._rust import DiapMcpService

logger = logging.getLogger(__name__)


def create_server() -> Server:
    """创建 DIAP MCP 服务器实例"""
    server = Server("diap-mcp-server")
    diap_service = DiapMcpService()

    @server.list_tools()
    async def list_tools() -> list[Tool]:
        """列出所有可用的工具"""
        return [
            Tool(
                name="create_agent",
                description="创建一个新的去中心化智能体，生成 DID 和密钥对",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "智能体名称"
                        },
                        "ipfs_api": {
                            "type": "string",
                            "description": "IPFS API 地址（可选，默认使用本地节点）"
                        }
                    },
                    "required": ["name"]
                }
            ),
            Tool(
                name="upload_to_ipfs",
                description="上传内容到 IPFS 网络",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "要上传的 JSON 内容"
                        }
                    },
                    "required": ["content"]
                }
            ),
            Tool(
                name="get_from_ipfs",
                description="从 IPFS 网络获取内容",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "cid": {
                            "type": "string",
                            "description": "IPFS CID"
                        }
                    },
                    "required": ["cid"]
                }
            ),
            Tool(
                name="verify_agent",
                description="验证智能体身份（使用零知识证明）",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "did": {
                            "type": "string",
                            "description": "智能体 DID"
                        },
                        "proof": {
                            "type": "string",
                            "description": "零知识证明数据"
                        }
                    },
                    "required": ["did", "proof"]
                }
            )
        ]

    @server.call_tool()
    async def call_tool(name: str, arguments: Any) -> list[TextContent]:
        """调用工具"""
        try:
            if name == "create_agent":
                result = await diap_service.create_agent(
                    arguments["name"],
                    arguments.get("ipfs_api")
                )
                return [TextContent(
                    type="text",
                    text=f"智能体创建成功:\n"
                         f"名称: {result['name']}\n"
                         f"DID: {result['did']}\n"
                         f"PeerID: {result['peer_id']}"
                )]

            elif name == "upload_to_ipfs":
                result = await diap_service.upload_to_ipfs(arguments["content"])
                return [TextContent(
                    type="text",
                    text=f"上传成功:\nCID: {result['cid']}\n大小: {result['size']} bytes"
                )]

            elif name == "get_from_ipfs":
                content = await diap_service.get_from_ipfs(arguments["cid"])
                return [TextContent(
                    type="text",
                    text=f"获取成功:\n{content}"
                )]

            elif name == "verify_agent":
                # TODO: 实现验证逻辑
                return [TextContent(
                    type="text",
                    text="验证功能开发中..."
                )]

            else:
                raise ValueError(f"未知工具: {name}")

        except Exception as e:
            logger.error(f"工具调用失败: {e}")
            return [TextContent(
                type="text",
                text=f"错误: {str(e)}"
            )]

    return server


async def main():
    """主函数"""
    logging.basicConfig(level=logging.INFO)
    server = create_server()
    
    async with stdio_server() as (read_stream, write_stream):
        logger.info("DIAP MCP Server 启动成功")
        await server.run(
            read_stream,
            write_stream,
            server.create_initialization_options()
        )


if __name__ == "__main__":
    asyncio.run(main())
