/**
 * Iroh P2P 通信器
 * 基于 Iroh 真实 API 的 P2P 通信实现
 * 提供可靠的端到端通信，与 PubSub 系统互补
 * 
 * 已更新以适配 iroh 0.96.1 API
 */
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

// Iroh 核心组件 - 基于真实 API (0.96.1)
use iroh::Endpoint;
use iroh_base::{EndpointAddr, EndpointId, PublicKey};

/// Iroh 通信器配置
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
    /// 是否启用 NAT 穿透
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

/// Iroh 消息类型
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

/// Iroh 通信消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohMessage {
    /// 消息 ID
    pub message_id: String,
    /// 消息类型
    pub message_type: IrohMessageType,
    /// 发送者 DID
    pub from_did: String,
    /// 接收者 DID（可选，用于直接通信）
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

/// Iroh 连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConnection {
    /// 远程节点 ID
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

/// Iroh 通信器
pub struct IrohCommunicator {
    /// 网络端点
    endpoint: Endpoint,
    /// 配置
    _config: IrohConfig,
    /// 活跃连接（使用 EndpointId 作为键）
    connections: HashMap<EndpointId, IrohConnection>,
    /// 消息接收通道
    message_receiver: mpsc::UnboundedReceiver<IrohMessage>,
    /// 消息发送通道
    message_sender: mpsc::UnboundedSender<IrohMessage>,
    /// 节点地址
    node_addr: EndpointAddr,
}

// ALPN 是 Iroh 约定的应用协议
const ALPN: &[u8] = b"diap-iroh/communication/1";

impl IrohCommunicator {
    /// 创建新的 Iroh 通信器
    pub async fn new(config: IrohConfig) -> Result<Self> {
        log::info!("🚀 创建 Iroh 通信器");

        // 构建节点端点，配置 ALPN 支持
        let endpoint = Endpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| anyhow!("Failed to bind endpoint: {}", e))?;

        // 获取本地节点地址
        let node_addr = endpoint.addr();

        // 创建消息通道
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        log::info!("✅ Iroh 通信器创建成功，节点 ID: {}", node_addr.id);

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
    pub fn get_node_addr(&self) -> String {
        format!("{}", self.node_addr.id)
    }

    /// 连接到远程节点（使用 EndpointAddr 对象）
    pub async fn connect_to_node_with_addr(&mut self, remote_addr: EndpointAddr) -> Result<String> {
        let remote_node_id = remote_addr.id.to_string();
        let node_addr_str = format!("{}", remote_addr.id);

        log::info!("🔗 连接到节点：{}", node_addr_str);
        log::debug!("   直接地址数量：{}", remote_addr.addrs.len());
        log::debug!("   中继 URL: {:?}", remote_addr.addrs.iter().find_map(|addr| {
            match addr {
                iroh_base::TransportAddr::Relay(url) => Some(url),
                _ => None,
            }
        }));

        // 获取连接超时配置（默认 30 秒）
        let timeout_secs = self._config.connection_timeout.unwrap_or(30);
        let timeout_duration = Duration::from_secs(timeout_secs);

        // 连接到目标节点，使用配置的超时时间
        let connect_future = self.endpoint.connect(remote_addr.clone(), ALPN);
        let _conn = tokio::time::timeout(timeout_duration, connect_future)
            .await
            .map_err(|_| anyhow!("Connection timeout after {} seconds", timeout_secs))?
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

        // 存储连接信息
        self.connections
            .insert(remote_addr.id, connection_info);

        log::info!("✅ 已连接到节点：{} ({})", remote_node_id, node_addr_str);
        Ok(remote_node_id)
    }

    /// 连接到远程节点（简化版本，需要预存的 NodeAddr）
    pub async fn connect_to_node(&mut self, node_id: &str) -> Result<String> {
        log::info!("🔗 连接到节点：{}", node_id);

        // 这里简化处理，实际应用中需要从 discovery 服务或缓存中获取 NodeAddr
        return Err(anyhow!("Please use connect_to_node_with_addr() with a proper NodeAddr object. NodeAddr cannot be parsed from string."));
    }

    /// 断开连接
    pub async fn disconnect_from_node(&mut self, node_id: &str) -> Result<()> {
        // 尝试解析 node_id
        if let Ok(node_id_key) = node_id.parse::<PublicKey>() {
            if let Some(connection) = self.connections.remove(&node_id_key) {
                log::info!(
                    "🔌 已断开与节点的连接：{} ({})",
                    node_id,
                    connection.remote_addr
                );
            }
        }
        Ok(())
    }

    /// 发送消息到指定节点
    pub async fn send_message(&self, node_id: &str, _message: IrohMessage) -> Result<()> {
        // 尝试解析 node_id
        if let Ok(node_id_key) = node_id.parse::<PublicKey>() {
            if let Some(_connection) = self.connections.get(&node_id_key) {
                // 需要从连接中重建 EndpointAddr，这里简化处理
                return Err(anyhow!("Need EndpointAddr to send message, not just EndpointId"));
            }
        }
        Err(anyhow!("节点未连接：{}", node_id))
    }

    /// 使用 EndpointAddr 对象发送消息到指定节点
    pub async fn send_message_with_addr(
        &self,
        remote_addr: EndpointAddr,
        message: IrohMessage,
    ) -> Result<()> {
        // 序列化消息
        let message_data = serde_json::to_vec(&message)?;

        // 计算 BLAKE3 哈希用于验证
        let hash = blake3::hash(&message_data);
        let data_hash = hash.to_string();

        // 连接到目标节点并建立 QUIC 双向流
        let conn = self
            .endpoint
            .connect(remote_addr, ALPN)
            .await
            .map_err(|e| anyhow!("Failed to connect for message sending: {}", e))?;
        let (mut send_stream, _recv_stream) = conn
            .open_bi()
            .await
            .map_err(|e| anyhow!("Failed to open bidirectional stream: {}", e))?;

        // 发送数据
        send_stream
            .write_all(&message_data)
            .await
            .map_err(|e| anyhow!("Failed to write message data: {}", e))?;
        send_stream
            .finish()
            .map_err(|e| anyhow!("Failed to finish stream: {}", e))?;

        log::debug!(
            "📤 消息已发送 (消息 ID: {}, 哈希：{})",
            message.message_id,
            data_hash
        );
        Ok(())
    }

    /// 创建认证请求消息
    pub fn create_auth_request(
        &self,
        from_did: &str,
        to_did: &str,
        challenge: &str,
    ) -> IrohMessage {
        let mut metadata = HashMap::new();
        metadata.insert("challenge".to_string(), challenge.to_string());

        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::AuthRequest,
            from_did: from_did.to_string(),
            to_did: Some(to_did.to_string()),
            content: format!("认证请求：{}", challenge),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            metadata,
        }
    }

    /// 创建认证响应消息
    pub fn create_auth_response(
        &self,
        from_did: &str,
        to_did: &str,
        response: &str,
    ) -> IrohMessage {
        let mut metadata = HashMap::new();
        metadata.insert("response".to_string(), response.to_string());

        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::AuthResponse,
            from_did: from_did.to_string(),
            to_did: Some(to_did.to_string()),
            content: format!("认证响应：{}", response),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
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
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            metadata: HashMap::new(),
        }
    }

    /// 创建自定义消息
    pub fn create_custom_message(
        &self,
        from_did: &str,
        to_did: Option<&str>,
        content: &str,
        message_type: &str,
    ) -> IrohMessage {
        IrohMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: IrohMessageType::Custom(message_type.to_string()),
            from_did: from_did.to_string(),
            to_did: to_did.map(|s| s.to_string()),
            content: content.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            metadata: HashMap::new(),
        }
    }

    /// 获取活跃连接列表
    pub fn get_connections(&self) -> HashMap<String, &IrohConnection> {
        self.connections
            .iter()
            .map(|(k, conn): (&EndpointId, &IrohConnection)| (k.to_string(), conn))
            .collect()
    }

    /// 检查连接状态
    pub fn is_connected(&self, node_id: &str) -> bool {
        if let Ok(node_id_key) = node_id.parse::<PublicKey>() {
            self.connections.get(&node_id_key).map_or(false, |conn| conn.connected)
        } else {
            false
        }
    }

    /// 获取连接统计信息
    pub fn get_connection_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert(
            "total_connections".to_string(),
            self.connections.len() as u64,
        );
        stats.insert(
            "active_connections".to_string(),
            self.connections
                .values()
                .filter(|conn| conn.connected)
                .count() as u64,
        );
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
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    signature: None,
                    metadata: HashMap::new(),
                };

                if let Err(e) = message_sender.send(heartbeat) {
                    log::error!("发送心跳失败：{}", e);
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
        log::info!("🎧 启动 Iroh 消息监听器");

        // 监听传入的连接
        while let Some(conn_future) = self.endpoint.accept().await {
            let conn = conn_future
                .await
                .map_err(|e| anyhow!("Failed to accept connection: {}", e))?;

            let remote_node_id = conn.remote_id();
            log::info!("📨 新连接建立，节点 ID: {}", remote_node_id);

            // 处理传入的双向流
            if let Ok((mut send_stream, mut recv_stream)) = conn.accept_bi().await {
                log::info!("📡 接受双向流");

                // 读取消息数据
                if let Ok(data) = recv_stream.read_to_end(1024).await {
                    log::info!("📥 收到消息：{} 字节", data.len());

                    // 反序列化消息
                    if let Ok(message) = serde_json::from_slice::<IrohMessage>(&data) {
                        log::info!(
                            "📨 收到消息：{} 来自节点：{}",
                            message.message_id,
                            remote_node_id
                        );

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
                send_stream
                    .finish()
                    .map_err(|e| log::error!("Failed to finish stream: {}", e))
                    .ok();
            }
        }

        Ok(())
    }

    /// 关闭通信器
    pub async fn shutdown(&mut self) -> Result<()> {
        // 断开所有连接
        let node_ids: Vec<EndpointId> = self.connections.keys().cloned().collect();
        for node_id in node_ids {
            self.connections.remove(&node_id);
        }

        // 关闭消息通道
        drop(self.message_sender.clone());

        log::info!("🔌 Iroh 通信器已关闭");
        Ok(())
    }

    /// 获取节点地址对象
    pub fn get_node_addr_object(&self) -> EndpointAddr {
        self.node_addr.clone()
    }

    /// 获取连接的节点列表
    pub fn get_connected_nodes(&self) -> Vec<String> {
        self.connections.keys().map(|k: &EndpointId| k.to_string()).collect()
    }

    /// 检查节点是否已连接
    pub fn is_node_connected(&self, node_id: &str) -> bool {
        if let Ok(node_id_key) = node_id.parse::<PublicKey>() {
            self.connections.contains_key(&node_id_key)
        } else {
            false
        }
    }
}

impl Drop for IrohCommunicator {
    fn drop(&mut self) {
        log::debug!("🧹 Iroh 通信器正在清理资源");
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
