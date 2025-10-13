// DIAP Rust SDK - Iroh节点接口（预留）
// Iroh是下一代P2P网络协议，提供更高效的数据传输
// 当前为预留接口，完整实现将在后续版本

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Iroh节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConfig {
    /// 节点名称
    pub node_name: String,
    
    /// 监听地址
    pub listen_addr: Option<String>,
    
    /// Bootstrap节点
    pub bootstrap_nodes: Vec<String>,
    
    /// 是否启用NAT穿透
    pub enable_nat_traversal: bool,
}

/// Iroh节点（预留）
#[cfg(feature = "iroh")]
pub struct IrohNode {
    config: IrohConfig,
    // iroh_net的实际实现将在这里
}

#[cfg(feature = "iroh")]
impl IrohNode {
    /// 创建新的Iroh节点
    pub async fn new(config: IrohConfig) -> Result<Self> {
        log::info!("🚀 创建Iroh节点: {}", config.node_name);
        log::warn!("⚠️  Iroh功能当前为预留状态");
        
        Ok(Self { config })
    }
    
    /// 启动节点
    pub async fn start(&mut self) -> Result<()> {
        log::info!("启动Iroh节点...");
        // TODO: 实现iroh-net的实际启动逻辑
        Err(anyhow::anyhow!("Iroh功能尚未实现，将在v0.3.0中添加"))
    }
    
    /// 连接到其他节点
    pub async fn connect(&mut self, _peer_addr: &str) -> Result<()> {
        Err(anyhow::anyhow!("Iroh功能尚未实现"))
    }
    
    /// 发送数据
    pub async fn send_data(&self, _peer_id: &str, _data: &[u8]) -> Result<()> {
        Err(anyhow::anyhow!("Iroh功能尚未实现"))
    }
    
    /// 接收数据
    pub async fn receive_data(&self) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!("Iroh功能尚未实现"))
    }
}

/// Iroh节点占位符（不启用iroh feature时）
#[cfg(not(feature = "iroh"))]
pub struct IrohNode {
    _phantom: std::marker::PhantomData<()>,
}

#[cfg(not(feature = "iroh"))]
impl IrohNode {
    /// 创建占位符节点
    pub async fn new(_config: IrohConfig) -> Result<Self> {
        log::warn!("⚠️  Iroh功能未启用（需要启用'iroh' feature）");
        Err(anyhow::anyhow!(
            "Iroh功能未启用。请在Cargo.toml中启用'iroh' feature:\n\
             diap-rs-sdk = {{ version = \"0.2\", features = [\"iroh\"] }}"
        ))
    }
}

/// Iroh辅助函数
pub mod helpers {
    /// 检查Iroh功能是否可用
    pub fn is_iroh_available() -> bool {
        cfg!(feature = "iroh")
    }
    
    /// 获取Iroh功能状态信息
    pub fn get_iroh_status() -> String {
        if is_iroh_available() {
            "Iroh功能已启用（预留接口）".to_string()
        } else {
            "Iroh功能未启用".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_iroh_status() {
        let status = super::helpers::get_iroh_status();
        println!("Iroh状态: {}", status);
        assert!(!status.is_empty());
    }
    
    #[tokio::test]
    #[ignore] // 因为Iroh功能未完全实现
    async fn test_iroh_node_creation() {
        use super::*;
        
        let config = IrohConfig {
            node_name: "test-node".to_string(),
            listen_addr: Some("/ip4/0.0.0.0/tcp/4000".to_string()),
            bootstrap_nodes: vec![],
            enable_nat_traversal: true,
        };
        
        let result = IrohNode::new(config).await;
        
        if cfg!(feature = "iroh") {
            // 如果启用了iroh feature，应该能创建
            assert!(result.is_ok());
        } else {
            // 否则应该返回错误
            assert!(result.is_err());
        }
    }
}

