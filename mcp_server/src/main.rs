use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use diap_rs_sdk::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// MCP 请求/响应结构
#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
}

// 应用状态
struct AppState {
    auth_manager: RwLock<AgentAuthManager>,
    ipfs_client: RwLock<Option<IpfsClient>>,
    noir_manager: RwLock<Option<UniversalNoirManager>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    log::info!("启动 DIAP MCP 服务器...");
    
    // 初始化状态
    let auth_manager = AgentAuthManager::new().await?;
    
    let state = Arc::new(AppState {
        auth_manager: RwLock::new(auth_manager),
        ipfs_client: RwLock::new(None),
        noir_manager: RwLock::new(None),
    });
    
    // 构建路由
    let app = Router::new()
        .route("/", get(health_check))
        .route("/mcp", post(handle_mcp_request))
        .with_state(state);
    
    let addr = "0.0.0.0:3000";
    log::info!("MCP 服务器运行在 http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> &'static str {
    "DIAP MCP Server is running"
}

async fn handle_mcp_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<McpRequest>,
) -> Json<McpResponse> {
    log::info!("收到 MCP 请求: {}", req.method);
    
    let result = match req.method.as_str() {
        "initialize" => handle_initialize(&state).await,
        "tools/list" => handle_list_tools().await,
        "tools/call" => handle_tool_call(&state, req.params).await,
        _ => Err(anyhow::anyhow!("未知方法: {}", req.method)),
    };
    
    match result {
        Ok(value) => Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(value),
            error: None,
        }),
        Err(e) => Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: None,
            error: Some(McpError {
                code: -32603,
                message: e.to_string(),
            }),
        }),
    }
}
