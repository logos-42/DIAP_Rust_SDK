// DIAP Rust SDK - P2P通信模块
// 完整的点对点通信实现，支持请求-响应模式和消息签名

use anyhow::{Context, Result};
use libp2p::{
    Multiaddr, PeerId, 
    request_response::{
        RequestResponse, Config as RequestResponseConfig, Event as RequestResponseEvent, ResponseChannel,
        ProtocolSupport, Codec as RequestResponseCodec, Message as RequestResponseMessage,
    },
    swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
    tcp::Config as TcpConfig,
    noise::Config as NoiseConfig,
    yamux::Config as YamuxConfig,
    Transport,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use futures::StreamExt;
use uuid::Uuid;

use crate::libp2p_identity::LibP2PIdentity;
use crate::key_manager::KeyPair;

/// DIAP协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPMessage {
    /// 消息ID
    pub message_id: String,
    
    /// 消息类型
    pub msg_type: String,
    
    /// 发送者DID
    pub from: String,
    
    /// 接收者DID
    pub to: String,
    
    /// 消息内容
    pub content: serde_json::Value,
    
    /// 时间戳
    pub timestamp: u64,
    
    /// nonce（防重放）
    pub nonce: String,
    
    /// 签名
    pub signature: String,
    
    /// 请求ID（用于请求-响应关联）
    pub request_id: Option<String>,
    
    /// 响应ID（用于请求-响应关联）
    pub response_id: Option<String>,
}

/// DIAP协议响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPResponse {
    /// 响应ID
    pub response_id: String,
    
    /// 请求ID（关联的请求）
    pub request_id: String,
    
    /// 响应类型
    pub response_type: String,
    
    /// 是否成功
    pub success: bool,
    
    /// 响应内容
    pub content: serde_json::Value,
    
    /// 错误信息（如果失败）
    pub error: Option<String>,
    
    /// 时间戳
    pub timestamp: u64,
    
    /// 签名
    pub signature: String,
}

/// DIAP网络行为（用于P2P通信）
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "DIAPP2PEvent")]
pub struct DIAPP2PBehaviour {
    /// 请求-响应协议
    pub request_response: RequestResponse<DIAPCodec>,
}

/// DIAP P2P事件
#[derive(Debug)]
pub enum DIAPP2PEvent {
    RequestResponse(RequestResponseEvent<DIAPMessage, DIAPResponse>),
}

impl From<RequestResponseEvent<DIAPMessage, DIAPResponse>> for DIAPP2PEvent {
    fn from(event: RequestResponseEvent<DIAPMessage, DIAPResponse>) -> Self {
        DIAPP2PEvent::RequestResponse(event)
    }
}

/// DIAP编解码器
#[derive(Clone)]
pub struct DIAPCodec;

impl RequestResponseCodec for DIAPCodec {
    type Protocol = String;
    type Request = DIAPMessage;
    type Response = DIAPResponse;

    fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        // 使用block_on来执行异步操作
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut buffer = Vec::new();
            futures::AsyncReadExt::read_to_end(io, &mut buffer).await?;
            let message: DIAPMessage = serde_json::from_slice(&buffer)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(message)
        })
    }

    fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        // 使用block_on来执行异步操作
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut buffer = Vec::new();
            futures::AsyncReadExt::read_to_end(io, &mut buffer).await?;
            let response: DIAPResponse = serde_json::from_slice(&buffer)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(response)
        })
    }

    fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, req: Self::Request) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        // 使用block_on来执行异步操作
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let data = serde_json::to_vec(&req)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            futures::AsyncWriteExt::write_all(io, &data).await?;
            futures::AsyncWriteExt::flush(io).await?;
            Ok(())
        })
    }

    fn write_response<T>(&mut self, _: &Self::Protocol, io: &mut T, res: Self::Response) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        // 使用block_on来执行异步操作
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let data = serde_json::to_vec(&res)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            futures::AsyncWriteExt::write_all(io, &data).await?;
            futures::AsyncWriteExt::flush(io).await?;
            Ok(())
        })
    }
}

/// P2P通信器
pub struct P2PCommunicator {
    /// Swarm实例
    swarm: Swarm<DIAPP2PBehaviour>,
    
    /// 事件接收器
    event_receiver: mpsc::UnboundedReceiver<DIAPP2PEvent>,
    
    /// 事件发送器
    event_sender: mpsc::UnboundedSender<DIAPP2PEvent>,
    
    /// 本地密钥对
    keypair: KeyPair,
    
    /// 本地DID
    local_did: String,
    
    /// 待响应的请求
    pending_requests: HashMap<String, ResponseChannel<DIAPResponse>>,
    
    /// 请求超时时间
    request_timeout: Duration,
}

impl P2PCommunicator {
    /// 创建新的P2P通信器
    pub async fn new(identity: LibP2PIdentity, keypair: KeyPair) -> Result<Self> {
        log::info!("创建P2P通信器");
        log::info!("  PeerID: {}", identity.peer_id());
        log::info!("  DID: {}", keypair.did);
        
        // 创建传输层
        let transport = TcpConfig::new()
            .upgrade(libp2p::upgrade::Version::V1Lazy)
            .authenticate(NoiseConfig::xx(identity.keypair().clone()))
            .multiplex(YamuxConfig::default())
            .boxed();
        
        // 创建请求-响应协议
        let protocol = "/diap/p2p/1.0.0".to_string();
        let request_response_config = RequestResponseConfig::default();
        let request_response = RequestResponse::new(
            [(protocol.clone(), ProtocolSupport::Full)],
            request_response_config,
        );
        
        // 创建网络行为
        let behaviour = DIAPP2PBehaviour {
            request_response,
        };
        
        // 创建事件通道
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        // 创建Swarm
        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, *identity.peer_id())
            .build();
        
        // 启动事件循环
        tokio::spawn({
            let sender = event_sender.clone();
            let mut swarm_clone = swarm.clone();
            
            async move {
                loop {
                    match swarm_clone.select_next_some().await {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            log::info!("📡 新监听地址: {}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            log::info!("🔗 连接建立: {}", peer_id);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            log::info!("❌ 连接关闭: {}", peer_id);
                        }
                        event => {
                            if let Err(e) = sender.send(event.into()) {
                                log::error!("发送P2P事件失败: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        
        Ok(Self {
            swarm,
            event_receiver,
            event_sender,
            keypair,
            local_did: keypair.did.clone(),
            pending_requests: HashMap::new(),
            request_timeout: Duration::from_secs(30),
        })
    }
    
    /// 启动监听
    pub fn listen(&mut self, addr: &str) -> Result<()> {
        let multiaddr: Multiaddr = addr.parse()
            .context("解析监听地址失败")?;
        
        self.swarm.listen_on(multiaddr)?;
        log::info!("✓ 添加监听地址: {}", addr);
        
        Ok(())
    }
    
    /// 连接到节点
    pub fn dial(&mut self, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        self.swarm.dial(addr)?;
        log::info!("📞 连接到节点: {}", peer_id);
        Ok(())
    }
    
    /// 发送请求消息
    pub async fn send_request(
        &mut self,
        peer_id: PeerId,
        message_type: &str,
        content: serde_json::Value,
        target_did: &str,
    ) -> Result<String> {
        let request_id = Uuid::new_v4().to_string();
        let message_id = Uuid::new_v4().to_string();
        let nonce = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // 创建消息
        let mut message = DIAPMessage {
            message_id: message_id.clone(),
            msg_type: message_type.to_string(),
            from: self.local_did.clone(),
            to: target_did.to_string(),
            content,
            timestamp,
            nonce,
            signature: String::new(), // 稍后签名
            request_id: Some(request_id.clone()),
            response_id: None,
        };
        
        // 签名消息
        message.signature = self.sign_message(&message)?;
        
        // 发送请求
        self.swarm.behaviour_mut().request_response.send_request(peer_id, message);
        
        log::info!("📤 发送请求到 {}: {}", peer_id, request_id);
        Ok(request_id)
    }
    
    /// 发送响应消息
    pub async fn send_response(
        &mut self,
        channel: ResponseChannel<DIAPResponse>,
        request_id: &str,
        success: bool,
        content: serde_json::Value,
        error: Option<String>,
    ) -> Result<()> {
        let response_id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // 创建响应
        let mut response = DIAPResponse {
            response_id: response_id.clone(),
            request_id: request_id.to_string(),
            response_type: "response".to_string(),
            success,
            content,
            error,
            timestamp,
            signature: String::new(), // 稍后签名
        };
        
        // 签名响应
        response.signature = self.sign_response(&response)?;
        
        // 发送响应
        self.swarm.behaviour_mut().request_response.send_response(channel, response);
        
        log::info!("📤 发送响应: {}", response_id);
        Ok(())
    }
    
    /// 处理事件
    pub async fn handle_events(&mut self) -> Result<()> {
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                DIAPP2PEvent::RequestResponse(RequestResponseEvent::Message { message, .. }) => {
                    match message {
                        RequestResponseMessage::Request { request, channel, .. } => {
                            log::info!("📨 收到请求: {} from {}", request.message_id, request.from);
                            
                            // 验证消息签名
                            if self.verify_message(&request)? {
                                // 处理请求
                                self.handle_request(request, channel).await?;
                            } else {
                                log::warn!("❌ 请求签名验证失败");
                            }
                        }
                        RequestResponseMessage::Response { response, .. } => {
                            log::info!("📨 收到响应: {}", response.response_id);
                            
                            // 验证响应签名
                            if self.verify_response(&response)? {
                                // 处理响应
                                self.handle_response(response).await?;
                            } else {
                                log::warn!("❌ 响应签名验证失败");
                            }
                        }
                    }
                }
                DIAPP2PEvent::RequestResponse(RequestResponseEvent::OutboundFailure { error, .. }) => {
                    log::error!("❌ 请求失败: {:?}", error);
                }
                DIAPP2PEvent::RequestResponse(RequestResponseEvent::InboundFailure { error, .. }) => {
                    log::error!("❌ 响应失败: {:?}", error);
                }
            }
        }
        
        Ok(())
    }
    
    /// 处理请求
    async fn handle_request(&mut self, request: DIAPMessage, channel: ResponseChannel<DIAPResponse>) -> Result<()> {
        log::info!("🔧 处理请求: {}", request.msg_type);
        
        // 根据消息类型处理请求
        let (success, content, error) = match request.msg_type.as_str() {
            "ping" => {
                (true, serde_json::json!({"message": "pong"}), None)
            }
            "get_info" => {
                let info = serde_json::json!({
                    "did": self.local_did,
                    "peer_id": self.swarm.local_peer_id().to_base58(),
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
                });
                (true, info, None)
            }
            _ => {
                (false, serde_json::Value::Null, Some("未知消息类型".to_string()))
            }
        };
        
        // 发送响应
        if let Some(request_id) = &request.request_id {
            self.send_response(channel, request_id, success, content, error).await?;
        }
        
        Ok(())
    }
    
    /// 处理响应
    async fn handle_response(&mut self, response: DIAPResponse) -> Result<()> {
        log::info!("🔧 处理响应: {} (成功: {})", response.response_id, response.success);
        
        // 这里可以添加响应处理逻辑
        // 例如：更新状态、触发回调等
        
        Ok(())
    }
    
    /// 签名消息
    fn sign_message(&self, message: &DIAPMessage) -> Result<String> {
        use ed25519_dalek::{SigningKey, Signer};
        
        let signing_key = SigningKey::from_bytes(&self.keypair.private_key);
        
        // 创建签名数据
        let sign_data = format!("{}{}{}{}{}", 
            message.message_id, 
            message.msg_type, 
            message.from, 
            message.to, 
            message.nonce
        );
        
        let signature = signing_key.sign(sign_data.as_bytes());
        Ok(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()))
    }
    
    /// 签名响应
    fn sign_response(&self, response: &DIAPResponse) -> Result<String> {
        use ed25519_dalek::{SigningKey, Signer};
        
        let signing_key = SigningKey::from_bytes(&self.keypair.private_key);
        
        // 创建签名数据
        let sign_data = format!("{}{}{}{}", 
            response.response_id, 
            response.request_id, 
            response.response_type, 
            response.success
        );
        
        let signature = signing_key.sign(sign_data.as_bytes());
        Ok(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()))
    }
    
    /// 验证消息签名
    fn verify_message(&self, message: &DIAPMessage) -> Result<bool> {
        use ed25519_dalek::{VerifyingKey, Verifier, Signature};
        
        // 从DID提取公钥（简化实现）
        let public_key_bytes = self.keypair.public_key_bytes();
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;
        
        // 创建签名数据
        let sign_data = format!("{}{}{}{}{}", 
            message.message_id, 
            message.msg_type, 
            message.from, 
            message.to, 
            message.nonce
        );
        
        let signature_bytes = base64::engine::general_purpose::STANDARD.decode(&message.signature)?;
        let signature = Signature::from_bytes(&signature_bytes)?;
        
        Ok(verifying_key.verify(sign_data.as_bytes(), &signature).is_ok())
    }
    
    /// 验证响应签名
    fn verify_response(&self, response: &DIAPResponse) -> Result<bool> {
        use ed25519_dalek::{VerifyingKey, Verifier, Signature};
        
        // 从DID提取公钥（简化实现）
        let public_key_bytes = self.keypair.public_key_bytes();
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;
        
        // 创建签名数据
        let sign_data = format!("{}{}{}{}", 
            response.response_id, 
            response.request_id, 
            response.response_type, 
            response.success
        );
        
        let signature_bytes = base64::engine::general_purpose::STANDARD.decode(&response.signature)?;
        let signature = Signature::from_bytes(&signature_bytes)?;
        
        Ok(verifying_key.verify(sign_data.as_bytes(), &signature).is_ok())
    }
    
    /// 获取当前监听的多地址
    pub fn listeners(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }
    
    /// 获取本地PeerID
    pub fn local_peer_id(&self) -> &PeerId {
        self.swarm.local_peer_id()
    }
    
    /// 获取本地DID
    pub fn local_did(&self) -> &str {
        &self.local_did
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_communicator() {
        let identity = LibP2PIdentity::generate().unwrap();
        let communicator = P2PCommunicator::new(identity).await;
        assert!(communicator.is_ok());
    }
    
    #[tokio::test]
    async fn test_listen() {
        let identity = LibP2PIdentity::generate().unwrap();
        let mut communicator = P2PCommunicator::new(identity).await.unwrap();
        
        let result = communicator.listen("/ip4/127.0.0.1/tcp/4001");
        assert!(result.is_ok());
    }
}
