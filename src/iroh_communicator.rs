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
    _config: IrohConfig,
    /// 活跃连接（使用NodeAddr作为键）
    connections: HashMap<String, (IrohConnection, NodeAddr)>,
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
            _config: config,
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

    /// 连接到远程节点（使用NodeAddr对象）
    pub async fn connect_to_node_with_addr(&mut self, remote_addr: NodeAddr) -> Result<String> {
        let remote_node_id = remote_addr.node_id.to_string();
        let node_addr_str = format!("{:?}", remote_addr.node_id);
        
        log::info!("🔗 连接到节点: {}", node_addr_str);

        // 连接到目标节点
        let _conn = self.endpoint.connect(remote_addr.clone(), ALPN).await
            .map_err(|e| anyhow!("Failed to connect to node: {}", e))?;
        
        // 记录连接
        let connection_info = IrohConnection {
            remote_node_id: remote_node_id.clone(),
            remote_addr: node_addr_str.clone(),
            connected: true,
            connected_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            data_hash: None,
        };

        // 存储连接信息和NodeAddr
        self.connections.insert(remote_node_id.clone(), (connection_info, remote_addr));

        log::info!("✅ 已连接到节点: {} ({})", remote_node_id, node_addr_str);
        Ok(remote_node_id)
    }

    /// 连接到远程节点（简化版本，需要预存的NodeAddr）
    pub async fn connect_to_node(&mut self, node_id: &str) -> Result<String> {
        log::info!("🔗 连接到节点: {}", node_id);

        // 这里简化处理，实际应用中需要从discovery服务或缓存中获取NodeAddr
        return Err(anyhow!("Please use connect_to_node_with_addr() with a proper NodeAddr object. NodeAddr cannot be parsed from string."));
    }

    /// 断开连接
    pub async fn disconnect_from_node(&mut self, node_id: &str) -> Result<()> {
        if let Some((mut connection, _node_addr)) = self.connections.remove(node_id) {
            connection.connected = false;
            log::info!("🔌 已断开与节点的连接: {} ({})", node_id, connection.remote_addr);
        }
        Ok(())
    }

    /// 发送消息到指定节点
    pub async fn send_message(&self, node_id: &str, message: IrohMessage) -> Result<()> {
        if let Some((_connection, node_addr)) = self.connections.get(node_id) {
            self.send_message_with_addr(node_addr.clone(), message).await
        } else {
            Err(anyhow!("节点未连接: {}", node_id))
        }
    }

    /// 使用NodeAddr对象发送消息到指定节点
    pub async fn send_message_with_addr(&self, remote_addr: NodeAddr, message: IrohMessage) -> Result<()> {
        // 序列化消息
        let message_data = serde_json::to_vec(&message)?;

        // 计算BLAKE3哈希用于验证
        let hash = blake3::hash(&message_data);
        let data_hash = hash.to_string();
        
        // 连接到目标节点并建立QUIC双向流
        let conn = self.endpoint.connect(remote_addr, ALPN).await
            .map_err(|e| anyhow!("Failed to connect for message sending: {}", e))?;
        let (mut send_stream, _recv_stream) = conn.open_bi().await
            .map_err(|e| anyhow!("Failed to open bidirectional stream: {}", e))?;
        
        // 发送数据
        send_stream.write_all(&message_data).await
            .map_err(|e| anyhow!("Failed to write message data: {}", e))?;
        send_stream.finish()
            .map_err(|e| anyhow!("Failed to finish stream: {}", e))?;

        log::debug!("📤 消息已发送 (消息ID: {}, 哈希: {})", 
                   message.message_id, data_hash);
        Ok(())
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
    pub fn get_connections(&self) -> HashMap<String, &IrohConnection> {
        self.connections.iter().map(|(k, (conn, _))| (k.clone(), conn)).collect()
    }

    /// 检查连接状态
    pub fn is_connected(&self, node_id: &str) -> bool {
        self.connections.get(node_id).map_or(false, |(conn, _)| conn.connected)
    }

    /// 获取连接统计信息
    pub fn get_connection_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("total_connections".to_string(), self.connections.len() as u64);
        stats.insert("active_connections".to_string(), 
            self.connections.values().filter(|(conn, _)| conn.connected).count() as u64);
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
        
        // 监听传入的连接
        while let Some(conn_future) = self.endpoint.accept().await {
            let conn_future = conn_future.await
                .map_err(|e| anyhow!("Failed to accept connection: {}", e))?;
            
            let remote_node_id = conn_future.remote_node_id();
            log::info!("📨 新连接建立，节点ID: {:?}", remote_node_id);
            
            // 处理传入的双向流
            if let Ok((mut send_stream, mut recv_stream)) = conn_future.accept_bi().await {
                log::info!("📡 接受双向流");
                
                // 读取消息数据
                if let Ok(data) = recv_stream.read_to_end(1024).await {
                    log::info!("📥 收到消息: {} 字节", data.len());
                    
                    // 反序列化消息
                    if let Ok(message) = serde_json::from_slice::<IrohMessage>(&data) {
                        log::info!("📨 收到消息: {} 来自节点: {:?}", 
                                  message.message_id, remote_node_id);
                        
                        // 通过内部通道发送消息
                        if let Err(e) = self.message_sender.send(message) {
                            log::error!("Failed to forward message: {}", e);
                        }
                        
                        // 发送响应
                        let response = b"Message received successfully!";
                        if let Err(e) = send_stream.write_all(response).await {
                            log::error!("Failed to send response: {}", e);
                        }
                    }
                }
                
                // 关闭流
                send_stream.finish().map_err(|e| log::error!("Failed to finish stream: {}", e)).ok();
            }
        }
        
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

    /// 获取节点地址对象
    pub fn get_node_addr_object(&self) -> NodeAddr {
        self.node_addr.clone()
    }

    /// 获取连接的节点列表
    pub fn get_connected_nodes(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    /// 检查节点是否已连接
    pub fn is_node_connected(&self, node_id: &str) -> bool {
        self.connections.contains_key(node_id)
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