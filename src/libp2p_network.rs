// DIAP Rust SDK - libp2p网络行为模块
// 完整的NetworkBehaviour实现，集成gossipsub、identify、ping、kad

use anyhow::{Context, Result};
use libp2p::{
    gossipsub::{self, Gossipsub, Config as GossipsubConfig, MessageId, TopicHash},
    identify::{Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    kad::{Kademlia, Config as KademliaConfig, Event as KademliaEvent},
    mdns::{Mdns, Config as MdnsConfig, Event as MdnsEvent},
    ping::{Ping, Config as PingConfig, Event as PingEvent},
    swarm::{
        NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent,
    },
    Multiaddr, PeerId, Transport,
    tcp::Config as TcpConfig,
    noise::Config as NoiseConfig,
    yamux::Config as YamuxConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use futures::StreamExt;

use crate::libp2p_identity::LibP2PIdentity;
use crate::pubsub_authenticator::{PubsubAuthenticator, AuthenticatedMessage};

/// DIAP网络行为
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "DIAPNetworkEvent")]
pub struct DIAPNetworkBehaviour {
    /// Gossipsub协议（PubSub）
    pub gossipsub: Gossipsub,
    
    /// 节点识别协议
    pub identify: Identify,
    
    /// Ping协议
    pub ping: Ping,
    
    /// Kademlia DHT
    pub kad: Kademlia,
    
    /// mDNS节点发现
    pub mdns: Mdns,
}

/// DIAP网络事件
#[derive(Debug)]
pub enum DIAPNetworkEvent {
    /// Gossipsub事件
    Gossipsub(gossipsub::Event),
    
    /// Identify事件
    Identify(IdentifyEvent),
    
    /// Ping事件
    Ping(PingEvent),
    
    /// Kademlia事件
    Kademlia(KademliaEvent),
    
    /// mDNS事件
    Mdns(MdnsEvent),
}

impl From<gossipsub::Event> for DIAPNetworkEvent {
    fn from(event: gossipsub::Event) -> Self {
        DIAPNetworkEvent::Gossipsub(event)
    }
}

impl From<IdentifyEvent> for DIAPNetworkEvent {
    fn from(event: IdentifyEvent) -> Self {
        DIAPNetworkEvent::Identify(event)
    }
}

impl From<PingEvent> for DIAPNetworkEvent {
    fn from(event: PingEvent) -> Self {
        DIAPNetworkEvent::Ping(event)
    }
}

impl From<KademliaEvent> for DIAPNetworkEvent {
    fn from(event: KademliaEvent) -> Self {
        DIAPNetworkEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for DIAPNetworkEvent {
    fn from(event: MdnsEvent) -> Self {
        DIAPNetworkEvent::Mdns(event)
    }
}

/// DIAP网络配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPNetworkConfig {
    /// 监听地址
    pub listen_addrs: Vec<String>,
    
    /// Bootstrap节点
    pub bootstrap_peers: Vec<String>,
    
    /// Gossipsub配置
    pub gossipsub_config: GossipsubConfig,
    
    /// 是否启用mDNS
    pub enable_mdns: bool,
    
    /// 是否启用Kademlia DHT
    pub enable_kad: bool,
    
    /// 协议版本
    pub protocol_version: String,
}

impl Default for DIAPNetworkConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec![
                "/ip4/0.0.0.0/tcp/4001".to_string(),
                "/ip6/::/tcp/4001".to_string(),
            ],
            bootstrap_peers: vec![],
            gossipsub_config: GossipsubConfig::default(),
            enable_mdns: true,
            enable_kad: true,
            protocol_version: "/diap/1.0.0".to_string(),
        }
    }
}

/// DIAP网络管理器
pub struct DIAPNetworkManager {
    /// Swarm实例
    swarm: Swarm<DIAPNetworkBehaviour>,
    
    /// 事件接收器
    event_receiver: mpsc::UnboundedReceiver<DIAPNetworkEvent>,
    
    /// 事件发送器
    event_sender: mpsc::UnboundedSender<DIAPNetworkEvent>,
    
    /// PubSub认证器
    pubsub_authenticator: Option<PubsubAuthenticator>,
    
    /// 订阅的主题
    subscribed_topics: HashMap<TopicHash, String>,
    
    /// 配置
    config: DIAPNetworkConfig,
}

impl DIAPNetworkManager {
    /// 创建新的DIAP网络管理器
    pub async fn new(
        identity: LibP2PIdentity,
        config: DIAPNetworkConfig,
        pubsub_authenticator: Option<PubsubAuthenticator>,
    ) -> Result<Self> {
        log::info!("🚀 创建DIAP网络管理器");
        log::info!("  PeerID: {}", identity.peer_id());
        
        // 创建传输层
        let transport = TcpConfig::new()
            .upgrade(libp2p::upgrade::Version::V1Lazy)
            .authenticate(NoiseConfig::xx(identity.keypair().clone()))
            .multiplex(YamuxConfig::default())
            .boxed();
        
        // 创建Gossipsub
        let gossipsub_config = config.gossipsub_config.clone();
        let gossipsub = Gossipsub::new(
            identity.keypair().clone(),
            gossipsub_config,
        )?;
        
        // 创建Identify
        let identify_config = IdentifyConfig::new(
            config.protocol_version.clone(),
            identity.keypair().public(),
        );
        let identify = Identify::new(identify_config);
        
        // 创建Ping
        let ping_config = PingConfig::new();
        let ping = Ping::new(ping_config);
        
        // 创建Kademlia
        let kad_config = KademliaConfig::default();
        let kad = Kademlia::new(identity.peer_id(), kad_config);
        
        // 创建mDNS
        let mdns = if config.enable_mdns {
            Mdns::new(MdnsConfig::default()).await?
        } else {
            Mdns::new(MdnsConfig::default()).await?
        };
        
        // 创建网络行为
        let behaviour = DIAPNetworkBehaviour {
            gossipsub,
            identify,
            ping,
            kad,
            mdns,
        };
        
        // 创建事件通道
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        // 创建Swarm
        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, *identity.peer_id())
            .build();
        
        // 添加监听地址
        for addr_str in &config.listen_addrs {
            let addr: Multiaddr = addr_str.parse()
                .with_context(|| format!("无效的监听地址: {}", addr_str))?;
            swarm.listen_on(addr)?;
            log::info!("✓ 添加监听地址: {}", addr_str);
        }
        
        // 添加Bootstrap节点
        for peer_str in &config.bootstrap_peers {
            let addr: Multiaddr = peer_str.parse()
                .with_context(|| format!("无效的Bootstrap地址: {}", peer_str))?;
            swarm.add_external_address(addr);
            log::info!("✓ 添加Bootstrap节点: {}", peer_str);
        }
        
        Ok(Self {
            swarm,
            event_receiver,
            event_sender,
            pubsub_authenticator,
            subscribed_topics: HashMap::new(),
            config,
        })
    }
    
    /// 启动网络管理器
    pub async fn start(&mut self) -> Result<()> {
        log::info!("🌐 启动DIAP网络管理器");
        
        // 启动事件循环
        tokio::spawn({
            let sender = self.event_sender.clone();
            let mut swarm = self.swarm.clone();
            
            async move {
                loop {
                    match swarm.select_next_some().await {
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
                                log::error!("发送网络事件失败: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        
        log::info!("✅ DIAP网络管理器启动完成");
        Ok(())
    }
    
    /// 订阅主题
    pub fn subscribe_topic(&mut self, topic: &str) -> Result<()> {
        let topic_hash = TopicHash::from_raw(topic);
        
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic_hash)?;
        self.subscribed_topics.insert(topic_hash, topic.to_string());
        
        log::info!("📢 订阅主题: {}", topic);
        Ok(())
    }
    
    /// 取消订阅主题
    pub fn unsubscribe_topic(&mut self, topic: &str) -> Result<()> {
        let topic_hash = TopicHash::from_raw(topic);
        
        self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic_hash)?;
        self.subscribed_topics.remove(&topic_hash);
        
        log::info!("📢 取消订阅主题: {}", topic);
        Ok(())
    }
    
    /// 发布消息到主题
    pub async fn publish_message(
        &mut self,
        topic: &str,
        content: &[u8],
    ) -> Result<MessageId> {
        let topic_hash = TopicHash::from_raw(topic);
        
        // 如果有认证器，创建认证消息
        let message_data = if let Some(ref authenticator) = self.pubsub_authenticator {
            let authenticated_msg = authenticator
                .create_authenticated_message(topic, content)
                .await?;
            serde_json::to_vec(&authenticated_msg)?
        } else {
            content.to_vec()
        };
        
        let message_id = self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic_hash, message_data)?;
        
        log::info!("📤 发布消息到主题 {}: {:?}", topic, message_id);
        Ok(message_id)
    }
    
    /// 处理网络事件
    pub async fn handle_events(&mut self) -> Result<()> {
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                DIAPNetworkEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id,
                    message,
                }) => {
                    log::info!("📨 收到Gossipsub消息: {} from {}", message_id, peer_id);
                    
                    // 如果有认证器，验证消息
                    if let Some(ref authenticator) = self.pubsub_authenticator {
                        if let Ok(authenticated_msg) = serde_json::from_slice::<AuthenticatedMessage>(&message.data) {
                            let verification = authenticator
                                .verify_authenticated_message(&authenticated_msg)
                                .await?;
                            
                            if verification.verified {
                                log::info!("✅ 消息验证通过: {}", authenticated_msg.from_did);
                                // 处理验证通过的消息
                                self.handle_verified_message(&authenticated_msg).await?;
                            } else {
                                log::warn!("❌ 消息验证失败: {:?}", verification.details);
                            }
                        } else {
                            log::warn!("❌ 无法解析认证消息");
                        }
                    } else {
                        // 没有认证器，直接处理消息
                        self.handle_raw_message(&message.data).await?;
                    }
                }
                DIAPNetworkEvent::Identify(IdentifyEvent::Received { peer_id, info }) => {
                    log::info!("🆔 收到节点信息: {} - {}", peer_id, info.protocol_version);
                }
                DIAPNetworkEvent::Ping(PingEvent { result: Ok(_), peer }) => {
                    log::debug!("🏓 Ping成功: {}", peer);
                }
                DIAPNetworkEvent::Ping(PingEvent { result: Err(e), peer }) => {
                    log::warn!("🏓 Ping失败: {} - {}", peer, e);
                }
                DIAPNetworkEvent::Kademlia(KademliaEvent::RoutingUpdated { peer, .. }) => {
                    log::info!("🗺️ DHT路由更新: {}", peer);
                }
                DIAPNetworkEvent::Mdns(MdnsEvent::Discovered(list)) => {
                    for (peer_id, multiaddr) in list {
                        log::info!("🔍 mDNS发现节点: {} at {}", peer_id, multiaddr);
                        self.swarm.add_external_address(multiaddr);
                    }
                }
                _ => {
                    log::debug!("📡 网络事件: {:?}", event);
                }
            }
        }
        
        Ok(())
    }
    
    /// 处理验证通过的消息
    async fn handle_verified_message(&mut self, message: &AuthenticatedMessage) -> Result<()> {
        log::info!("📝 处理验证消息: {} from {}", message.message_id, message.from_did);
        
        // 这里可以添加具体的消息处理逻辑
        // 例如：更新DID文档、同步状态等
        
        Ok(())
    }
    
    /// 处理原始消息
    async fn handle_raw_message(&mut self, data: &[u8]) -> Result<()> {
        log::info!("📝 处理原始消息: {} bytes", data.len());
        
        // 这里可以添加原始消息的处理逻辑
        
        Ok(())
    }
    
    /// 获取本地PeerID
    pub fn local_peer_id(&self) -> &PeerId {
        self.swarm.local_peer_id()
    }
    
    /// 获取监听地址
    pub fn listeners(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }
    
    /// 连接到节点
    pub fn dial(&mut self, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        self.swarm.dial(addr)?;
        log::info!("📞 连接到节点: {}", peer_id);
        Ok(())
    }
    
    /// 获取已订阅的主题列表
    pub fn subscribed_topics(&self) -> Vec<String> {
        self.subscribed_topics.values().cloned().collect()
    }
    
    /// 获取网络统计信息
    pub fn get_network_stats(&self) -> NetworkStats {
        NetworkStats {
            peer_id: self.local_peer_id().to_base58(),
            listeners: self.listeners().iter().map(|addr| addr.to_string()).collect(),
            subscribed_topics: self.subscribed_topics(),
            connected_peers: self.swarm.connected_peers().count(),
        }
    }
}

/// 网络统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub peer_id: String,
    pub listeners: Vec<String>,
    pub subscribed_topics: Vec<String>,
    pub connected_peers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_network_manager() {
        let identity = LibP2PIdentity::generate().unwrap();
        let config = DIAPNetworkConfig::default();
        
        let manager = DIAPNetworkManager::new(identity, config, None).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_subscribe_topic() {
        let identity = LibP2PIdentity::generate().unwrap();
        let config = DIAPNetworkConfig::default();
        
        let mut manager = DIAPNetworkManager::new(identity, config, None).await.unwrap();
        
        let result = manager.subscribe_topic("test-topic");
        assert!(result.is_ok());
        
        let topics = manager.subscribed_topics();
        assert!(topics.contains(&"test-topic".to_string()));
    }
}
