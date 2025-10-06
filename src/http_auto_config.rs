/**
 * ANP HTTP端口自动配置模块 - Rust版本
 * 提供端口自动分配、HTTP服务器自动启动等功能
 */

use std::net::{TcpListener, SocketAddr};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use warp::Filter;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use log::info;
use serde_json::Value;

// 类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPAutoConfigOptions {
    /// 是否自动启动HTTP服务器
    pub auto_start: Option<bool>,
    /// 是否自动分配端口
    pub auto_port: Option<bool>,
    /// 端口范围
    pub port_range: Option<(u16, u16)>,
    /// 主机地址
    pub host: Option<String>,
    /// 日志级别
    pub log_level: Option<String>,
    /// 自定义路由
    pub routes: Option<Vec<RouteConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub method: String,
    pub path: String,
    pub handler_type: String, // "json", "text", "html"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPConfig {
    pub port: u16,
    pub host: String,
    pub local_ip: String,
    pub endpoint: String,
    pub is_running: bool,
}

/**
 * HTTP端口自动配置结构体
 */
pub struct HTTPAutoConfig {
    options: HTTPAutoConfigOptions,
    auto_port: Option<u16>,
    local_ip: Option<String>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    is_running: Arc<RwLock<bool>>,
    routes: Arc<RwLock<HashMap<String, RouteConfig>>>,
    // 存储 DID 文档和 AD 文档
    pub did_document: Arc<RwLock<Option<Value>>>,
    pub ad_document: Arc<RwLock<Option<Value>>>,
}

impl HTTPAutoConfig {
    /// 创建新的HTTP自动配置实例
    pub fn new(options: HTTPAutoConfigOptions) -> Self {
        Self {
            options,
            auto_port: None,
            local_ip: None,
            server_handle: None,
            is_running: Arc::new(RwLock::new(false)),
            routes: Arc::new(RwLock::new(HashMap::new())),
            did_document: Arc::new(RwLock::new(None)),
            ad_document: Arc::new(RwLock::new(None)),
        }
    }
    
    /// 设置 DID 文档
    pub async fn set_did_document(&self, doc: Value) {
        *self.did_document.write().await = Some(doc);
    }
    
    /// 设置 AD 文档
    pub async fn set_ad_document(&self, doc: Value) {
        *self.ad_document.write().await = Some(doc);
    }

    /// 核心方法：自动配置HTTP服务器
    pub async fn auto_setup(&mut self) -> Result<HTTPConfig> {
        info!("🔄 HTTP自动配置: 开始配置...");
        
        // 步骤1: 自动分配端口
        if self.options.auto_port.unwrap_or(true) {
            self.auto_port = Some(self.find_available_port().await?);
            info!("✅ 自动分配端口: {}", self.auto_port.unwrap());
        } else {
            self.auto_port = Some(self.options.port_range.unwrap_or((3000, 4000)).0);
            info!("✅ 使用指定端口: {}", self.auto_port.unwrap());
        }
        
        // 步骤2: 获取本地IP
        self.local_ip = Some(self.get_local_ip().await?);
        info!("✅ 本地IP: {}", self.local_ip.as_ref().unwrap());
        
        // 步骤3: 启动HTTP服务器
        if self.options.auto_start.unwrap_or(true) {
            self.start_http_server().await?;
            info!("✅ HTTP服务器启动在端口: {}", self.auto_port.unwrap());
            
            // 步骤4: 配置路由
            self.setup_routes().await;
            info!("✅ 路由配置完成");
        }
        
        *self.is_running.write().await = true;
        info!("🎉 HTTP自动配置完成！");
        
        Ok(self.get_config().await)
    }

    /// 自动分配可用端口
    async fn find_available_port(&self) -> Result<u16> {
        let (start_port, end_port) = self.options.port_range.unwrap_or((3000, 4000));
        
        // 首先尝试指定范围内的端口
        for port in start_port..=end_port {
            if self.is_port_available(port).await? {
                return Ok(port);
            }
        }
        
        // 如果指定范围内没有可用端口，使用系统自动分配
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(port)
    }

    /// 检查端口是否可用
    async fn is_port_available(&self, port: u16) -> Result<bool> {
        let host = self.options.host.as_deref().unwrap_or("127.0.0.1");
        let addr = format!("{}:{}", host, port);
        
        match TcpListener::bind(&addr) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 获取本地IP地址
    async fn get_local_ip(&self) -> Result<String> {
        // 简化实现，返回localhost
        // 在实际应用中，这里应该获取真实的网络接口IP
        Ok("127.0.0.1".to_string())
    }

    /// 启动HTTP服务器
    async fn start_http_server(&mut self) -> Result<()> {
        let port = self.auto_port.unwrap();
        let host = self.options.host.as_deref().unwrap_or("127.0.0.1").to_string();
        let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
        
        // 克隆 Arc 引用用于路由
        let did_doc = self.did_document.clone();
        let ad_doc = self.ad_document.clone();
        
        // 健康检查路由
        let health_route = warp::path("health")
            .and(warp::get())
            .map(move || {
                warp::reply::json(&serde_json::json!({
                    "status": "healthy",
                    "port": port,
                    "host": "127.0.0.1",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            });

        // 配置信息路由
        let config_route = warp::path("config")
            .and(warp::get())
            .map(move || {
                let config = HTTPConfig {
                    port,
                    host: "127.0.0.1".to_string(),
                    local_ip: "127.0.0.1".to_string(),
                    endpoint: format!("http://127.0.0.1:{}", port),
                    is_running: true,
                };
                warp::reply::json(&config)
            });

        // DID 文档路由: /.well-known/did.json
        let did_doc_clone = did_doc.clone();
        let did_route = warp::path!(".well-known" / "did.json")
            .and(warp::get())
            .and_then(move || {
                let did_doc = did_doc_clone.clone();
                async move {
                    let doc = did_doc.read().await;
                    match doc.as_ref() {
                        Some(d) => Ok::<_, warp::Rejection>(warp::reply::json(d)),
                        None => {
                            let error = serde_json::json!({
                                "error": "DID document not available",
                                "message": "DID document has not been configured yet"
                            });
                            Ok(warp::reply::json(&error))
                        }
                    }
                }
            });

        // Agent Description 路由: /agents/{agent_id}/ad.json
        let ad_doc_clone = ad_doc.clone();
        let ad_route = warp::path!("agents" / String / "ad.json")
            .and(warp::get())
            .and_then(move |_agent_id: String| {
                let ad_doc = ad_doc_clone.clone();
                async move {
                    let doc = ad_doc.read().await;
                    match doc.as_ref() {
                        Some(d) => Ok::<_, warp::Rejection>(warp::reply::json(d)),
                        None => {
                            let error = serde_json::json!({
                                "error": "Agent description not available",
                                "message": "Agent description has not been configured yet"
                            });
                            Ok(warp::reply::json(&error))
                        }
                    }
                }
            });

        // ANP API 路由: /anp/api
        let anp_api_route = warp::path!("anp" / "api")
            .and(warp::post())
            .and(warp::body::json())
            .map(|body: serde_json::Value| {
                // 简单的回显响应，实际应用中应该处理 ANP 协议消息
                warp::reply::json(&serde_json::json!({
                    "response": "ANP message received",
                    "echo": body,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            });

        // 通用 404 路由
        let fallback_route = warp::any()
            .map(|| {
                warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "error": "Not Found",
                        "message": "The requested endpoint does not exist"
                    })),
                    warp::http::StatusCode::NOT_FOUND,
                )
            });

        // 组合所有路由
        let routes = health_route
            .or(config_route)
            .or(did_route)
            .or(ad_route)
            .or(anp_api_route)
            .or(fallback_route)
            .with(warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["content-type", "authorization"])
                .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]));

        // 启动服务器
        let server_handle = tokio::spawn(async move {
            info!("🚀 HTTP服务器启动在: {}", addr);
            warp::serve(routes).run(addr).await;
        });

        self.server_handle = Some(server_handle);
        Ok(())
    }

    /// 配置路由
    async fn setup_routes(&self) {
        if let Some(routes) = &self.options.routes {
            let mut routes_guard = self.routes.write().await;
            for route in routes {
                let key = format!("{}:{}", route.method, route.path);
                routes_guard.insert(key, route.clone());
            }
        }
    }

    /// 添加路由
    pub async fn add_route(&self, route: RouteConfig) {
        let method = route.method.clone();
        let path = route.path.clone();
        let mut routes_guard = self.routes.write().await;
        let key = format!("{}:{}", method, path);
        routes_guard.insert(key, route);
        info!("✅ 添加路由: {} {}", method, path);
    }

    /// 获取配置信息
    pub async fn get_config(&self) -> HTTPConfig {
        let port = self.auto_port.unwrap_or(0);
        let host = self.options.host.as_deref().unwrap_or("127.0.0.1").to_string();
        let local_ip = self.local_ip.as_deref().unwrap_or("127.0.0.1").to_string();
        let is_running = *self.is_running.read().await;

        HTTPConfig {
            port,
            host,
            local_ip: local_ip.clone(),
            endpoint: format!("http://{}:{}", local_ip, port),
            is_running,
        }
    }

    /// 获取服务端点
    pub fn get_endpoint(&self) -> Result<String> {
        if let (Some(local_ip), Some(port)) = (&self.local_ip, &self.auto_port) {
            Ok(format!("http://{}:{}", local_ip, port))
        } else {
            Err(anyhow::anyhow!("HTTP服务器未配置"))
        }
    }

    /// 停止服务
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            *self.is_running.write().await = false;
            info!("🛑 HTTP服务器已停止");
        }
        Ok(())
    }

    /// 检查是否正在运行
    pub async fn is_server_running(&self) -> bool {
        *self.is_running.read().await
    }
}

impl Default for HTTPAutoConfigOptions {
    fn default() -> Self {
        Self {
            auto_start: Some(true),
            auto_port: Some(true),
            port_range: Some((3000, 4000)),
            host: Some("127.0.0.1".to_string()),
            log_level: Some("info".to_string()),
            routes: Some(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_auto_config() {
        let options = HTTPAutoConfigOptions::default();
        let mut config = HTTPAutoConfig::new(options);
        
        let result = config.auto_setup().await;
        assert!(result.is_ok());
        
        let http_config = result.unwrap();
        assert!(http_config.port > 0);
        assert!(http_config.is_running);
        
        config.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_port_allocation() {
        let options = HTTPAutoConfigOptions {
            port_range: Some((3000, 3010)),
            ..Default::default()
        };
        let config = HTTPAutoConfig::new(options);
        
        let port = config.find_available_port().await.unwrap();
        assert!(port >= 3000 && port <= 3010);
    }
}
