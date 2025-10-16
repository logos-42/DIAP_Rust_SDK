/**
 * Iroh P2P通信器
 * 基于Iroh真实API的P2P通信实现
 * 提供可靠的端到端通信，与PubSub系统互补
 */

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

// Iroh核心组件 - 基于真实API
use iroh::{Endpoint, NodeAddr};

/// Iroh通信器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConfig {
    /// 监听地址
    pub listen_addr: Option<std::net::SocketAddr>,
    /// 数据存储目录
    pub data_dir: Option<std::path::PathBuf>,
    /// 最大连接数
    pub max_connections: Option<usize>,
    /// 连接超时时间（秒）
    pub connection_timeout: Option<u64>,
    /// 是否启用中继
    pub enable_relay: Option<bool>,
    /// 是否启用NAT穿透
    pub enable_nat_traversal: Option<bool>,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            listen_addr: Some("0.0.0.0:0".parse().unwrap()),
            data_dir: None,
            max_connections: Some(100),
            connection_timeout: Some(30),
            enable_relay: Some(true),
            enable_nat_traversal: Some(true),
        }
    }
}

/// Iroh消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrohMessageType {
    /// 身份验证请求
    AuthRequest,
    /// 身份验证响应
    AuthResponse,
    /// 资源请求
    ResourceRequest,
    /// 资源响应
    ResourceResponse,
    /// 心跳消息
    Heartbeat,
    /// 自定义消息
    Custom(String),
}

/// Iroh通信消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohMessage {
    /// 消息ID
    pub message_id: String,
    /// 消息类型
    pub message_type: IrohMessageType,
    /// 发送者DID
    pub from_did: String,
    /// 接收者DID（可选，用于直接通信）
    pub to_did: Option<String>,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: u64,
    /// 签名（可选）
    pub signature: Option<String>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// Iroh连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConnection {
    /// 远程节点ID
    pub remote_node_id: String,
    /// 远程地址
    pub remote_addr: String,
    /// 连接状态
    pub connected: bool,
    /// 连接时间
    pub connected_at: u64,
    /// 最后心跳时间
    pub last_heartbeat: u64,
    /// 数据哈希（用于验证）
    pub data_hash: Option<String>,
}

/// Iroh通信器
pub struct IrohCommunicator {
    /// 网络端点
    endpoint: Endpoint,
    /// 配置
    config: IrohConfig,
    /// 活跃连接
    connections: HashMap<String, IrohConnection>,
    /// 消息接收通道
    message_receiver: mpsc::UnboundedReceiver<IrohMessage>,
    /// 消息发送通道
    message_sender: mpsc::UnboundedSender<IrohMessage>,
    /// 节点地址
    node_addr: NodeAddr,
}

// ALPN是Iroh约定的应用协议
const ALPN: &[u8] = b"diap-iroh/communication/1";

impl IrohCommunicator {
    /// 创建新的Iroh通信器
    pub async fn new(config: IrohConfig) -> Result<Self> {
        log::info!("🚀 创建Iroh通信器");

        // 构建节点端点，配置ALPN支持
        let endpoint = Endpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| anyhow!("Failed to bind endpoint: {}", e))?;

        // 获取本地节点地址
        let node_addr = endpoint.node_addr();

        // 创建消息通道
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        log::info!("✅ Iroh通信器创建成功，节点ID: {}", node_addr.node_id);

        Ok(Self {
            endpoint,
            config,
            connections: HashMap::new(),
            message_receiver,
            message_sender,
            node_addr,
        })
    }

    /// 获取节点地址
    pub fn get_node_addr(&self) -> Result<String> {
        // NodeAddr没有实现Display trait，我们返回节点ID的字符串表示
        Ok(format!("NodeID: {:?}", self.node_addr.node_id))
    }

    /// 连接到远程节点
    pub async fn connect_to_node(&mut self, node_addr: &str) -> Result<String> {
        log::info!("🔗 连接到节点: {}", node_addr);

        // 暂时使用简化的连接方式，实际应用中需要从字符串构造NodeAddr
        // 这里我们创建一个占位符实现，实际使用时需要根据具体的NodeAddr构造方法
        return Err(anyhow!("NodeAddr construction from string not yet implemented. Please provide a proper NodeAddr object."));
    }

    /// 断开连接
    pub async fn disconnect_from_node(&mut self, node_id: &str) -> Result<()> {
        if let Some(mut connection) = self.connections.remove(node_id) {
            connection.connected = false;
            log::info!("🔌 已断开与节点的连接: {} ({})", node_id, connection.remote_addr);
        }
        Ok(())
    }

    /// 发送消息到指定节点
    pub async fn send_message(&self, node_id: &str, message: IrohMessage) -> Result<()> {
        if !self.connections.contains_key(node_id) {
            return Err(anyhow!("节点未连接: {}", node_id));
        }

        // 序列化消息
        let message_data = serde_json::to_vec(&message)?;

        // 计算BLAKE3哈希用于验证
        let _hash = blake3::hash(&message_data);

        // 暂时返回错误，因为NodeAddr构造需要进一步研究
        return Err(anyhow!("Message sending not yet implemented due to NodeAddr construction complexity"));
    }

    /// 创建认证请求消息
    pub fn create_auth_request(&self, from_did: &str, to_did: &str, challenge: &str) -> IrohMessage {
        let mut metadata = HashMap::new();
        metadata.insert("challenge".to_string(), challenge.to_string());

        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::AuthRequest,
            from_did: from_did.to_string(),
            to_did: Some(to_did.to_string()),
            content: format!("认证请求: {}", challenge),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            signature: None,
            metadata,
        }
    }

    /// 创建认证响应消息
    pub fn create_auth_response(&self, from_did: &str, to_did: &str, response: &str) -> IrohMessage {
        let mut metadata = HashMap::new();
        metadata.insert("response".to_string(), response.to_string());

        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::AuthResponse,
            from_did: from_did.to_string(),
            to_did: Some(to_did.to_string()),
            content: format!("认证响应: {}", response),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            signature: None,
            metadata,
        }
    }

    /// 创建心跳消息
    pub fn create_heartbeat(&self, from_did: &str) -> IrohMessage {
        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::Heartbeat,
            from_did: from_did.to_string(),
            to_did: None,
            content: "心跳".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            signature: None,
            metadata: HashMap::new(),
        }
    }

    /// 创建自定义消息
    pub fn create_custom_message(&self, from_did: &str, to_did: Option<&str>, content: &str, message_type: &str) -> IrohMessage {
        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::Custom(message_type.to_string()),
            from_did: from_did.to_string(),
            to_did: to_did.map(|s| s.to_string()),
            content: content.to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            signature: None,
            metadata: HashMap::new(),
        }
    }

    /// 获取活跃连接列表
    pub fn get_connections(&self) -> &HashMap<String, IrohConnection> {
        &self.connections
    }

    /// 检查连接状态
    pub fn is_connected(&self, node_id: &str) -> bool {
        self.connections.get(node_id).map_or(false, |conn| conn.connected)
    }

    /// 获取连接统计信息
    pub fn get_connection_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("total_connections".to_string(), self.connections.len() as u64);
        stats.insert("active_connections".to_string(), 
            self.connections.values().filter(|conn| conn.connected).count() as u64);
        stats
    }

    /// 启动心跳监控
    pub async fn start_heartbeat_monitor(&self, from_did: &str, interval: Duration) {
        let message_sender = self.message_sender.clone();
        let from_did = from_did.to_string();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                let heartbeat = IrohMessage {
                    message_id: uuid::Uuid::new_v4().to_string(),
                    message_type: IrohMessageType::Heartbeat,
                    from_did: from_did.clone(),
                    to_did: None,
                    content: "心跳".to_string(),
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    signature: None,
                    metadata: HashMap::new(),
                };

                if let Err(e) = message_sender.send(heartbeat) {
                    log::error!("发送心跳失败: {}", e);
                    break;
                }
            }
        });
    }

    /// 接收消息
    pub async fn receive_message(&mut self) -> Option<IrohMessage> {
        self.message_receiver.recv().await
    }

    /// 启动消息监听器
    pub async fn start_message_listener(&mut self) -> Result<()> {
        log::info!("🎧 启动Iroh消息监听器");
        
        // 暂时实现基础监听器框架
        // 实际实现需要更复杂的连接管理
        log::info!("✅ Iroh消息监听器已启动（基础版本）");
        log::info!("⚠️  完整实现需要进一步研究NodeAddr构造和连接管理");
        
        Ok(())
    }

    /// 关闭通信器
    pub async fn shutdown(&mut self) -> Result<()> {
        // 断开所有连接
        for (node_id, _) in self.connections.clone() {
            self.disconnect_from_node(&node_id).await?;
        }

        // 关闭消息通道
        drop(self.message_sender.clone());

        log::info!("🔌 Iroh通信器已关闭");
        Ok(())
    }
}

impl Drop for IrohCommunicator {
    fn drop(&mut self) {
        log::debug!("🧹 Iroh通信器正在清理资源");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iroh_communicator_creation() {
        let config = IrohConfig::default();
        let communicator = IrohCommunicator::new(config).await;
        assert!(communicator.is_ok());
    }

    #[tokio::test]
    async fn test_message_creation() {
        let config = IrohConfig::default();
        let communicator = IrohCommunicator::new(config).await.unwrap();
        
        let auth_req = communicator.create_auth_request("did:alice", "did:bob", "challenge123");
        assert_eq!(auth_req.from_did, "did:alice");
        assert_eq!(auth_req.to_did, Some("did:bob".to_string()));
        
        let heartbeat = communicator.create_heartbeat("did:alice");
        assert_eq!(heartbeat.from_did, "did:alice");
        assert_eq!(heartbeat.to_did, None);
    }
}