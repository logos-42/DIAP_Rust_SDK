// ANP Rust SDK - libp2p节点模块
// 运行libp2p P2P节点，处理网络通信

use anyhow::{Context, Result};
use libp2p::{
   
    Multiaddr,
    PeerId,
    
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::libp2p_identity::LibP2PIdentity;

/// libp2p节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// PeerID
    pub peer_id: String,
    
    /// 当前监听的多地址
    pub multiaddrs: Vec<String>,
    
    /// 支持的协议
    pub protocols: Vec<String>,
    
    /// 更新时间
    pub updated_at: String,
}

/// libp2p节点（简化版本）
pub struct LibP2PNode {
    /// PeerID
    peer_id: PeerId,
    
    /// 当前监听的地址
    listen_addrs: Vec<Multiaddr>,
}

impl LibP2PNode {
    /// 创建新的libp2p节点（简化实现）
    /// 注意：这是一个基础实现，完整的Swarm需要定义Behaviour
    pub fn new(identity: &LibP2PIdentity) -> Result<Self> {
        log::info!("创建libp2p节点");
        log::info!("  PeerID: {}", identity.peer_id());
        
        Ok(Self {
            peer_id: identity.peer_id().clone(),
            listen_addrs: Vec::new(),
        })
    }
    
    /// 添加监听地址
    pub fn add_listen_addr(&mut self, addr: &str) -> Result<()> {
        let multiaddr = Multiaddr::from_str(addr)
            .with_context(|| format!("无效的多地址: {}", addr))?;
        
        self.listen_addrs.push(multiaddr);
        log::info!("添加监听地址: {}", addr);
        
        Ok(())
    }
    
    /// 获取节点信息
    pub fn get_node_info(&self) -> NodeInfo {
        // 构造完整的多地址（包含PeerID）
        let multiaddrs: Vec<String> = self.listen_addrs
            .iter()
            .map(|addr| {
                // 添加PeerID到多地址
                format!("{}/p2p/{}", addr, self.peer_id)
            })
            .collect();
        
        NodeInfo {
            peer_id: self.peer_id.to_base58(),
            multiaddrs,
            protocols: vec!["/anp/1.0.0".to_string()],
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    /// 获取PeerID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}

// 注意：完整的libp2p Swarm实现需要定义NetworkBehaviour
// 这里提供一个基础版本用于获取节点信息
// 完整的P2P通信功能将在后续版本实现

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libp2p_identity::LibP2PIdentity;
    
    #[test]
    fn test_create_node() {
        let identity = LibP2PIdentity::generate().unwrap();
        let node = LibP2PNode::new(&identity).unwrap();
        
        assert_eq!(node.peer_id(), identity.peer_id());
    }
    
    #[test]
    fn test_add_listen_addr() {
        let identity = LibP2PIdentity::generate().unwrap();
        let mut node = LibP2PNode::new(&identity).unwrap();
        
        node.add_listen_addr("/ip4/0.0.0.0/tcp/4001").unwrap();
        node.add_listen_addr("/ip6/::/tcp/4001").unwrap();
        
        let info = node.get_node_info();
        assert_eq!(info.multiaddrs.len(), 2);
        assert!(info.multiaddrs[0].contains(&info.peer_id));
    }
    
    #[test]
    fn test_node_info() {
        let identity = LibP2PIdentity::generate().unwrap();
        let mut node = LibP2PNode::new(&identity).unwrap();
        
        node.add_listen_addr("/ip4/127.0.0.1/tcp/4001").unwrap();
        
        let info = node.get_node_info();
        
        assert!(!info.peer_id.is_empty());
        assert!(!info.multiaddrs.is_empty());
        assert_eq!(info.protocols, vec!["/anp/1.0.0"]);
        assert!(!info.updated_at.is_empty());
        
        println!("节点信息: {:?}", info);
    }
}
