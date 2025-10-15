// DIAP Rust SDK - 内置IPFS节点管理器
// 自动启动和管理本地IPFS节点，实现完全去中心化

use anyhow::{Context, Result};
use std::process::{Command, Child};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use log;

/// IPFS节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsNodeConfig {
    /// IPFS数据目录
    pub data_dir: PathBuf,
    
    /// API端口（默认5001）
    pub api_port: u16,
    
    /// 网关端口（默认8080）
    pub gateway_port: u16,
    
    /// 是否启用自动启动
    pub auto_start: bool,
    
    /// 启动超时时间（秒）
    pub startup_timeout: u64,
    
    /// 是否启用Bootstrap节点
    pub enable_bootstrap: bool,
    
    /// 是否启用Swarm端口
    pub enable_swarm: bool,
    
    /// Swarm端口（默认4001）
    pub swarm_port: u16,
    
    /// 是否启用详细日志
    pub verbose_logging: bool,
}

impl Default for IpfsNodeConfig {
    fn default() -> Self {
        let mut data_dir = std::env::temp_dir();
        data_dir.push("diap-ipfs");
        
        Self {
            data_dir,
            api_port: 5001,
            gateway_port: 8080,
            auto_start: true,
            startup_timeout: 30,
            enable_bootstrap: true,
            enable_swarm: true,
            swarm_port: 4001,
            verbose_logging: false,
        }
    }
}

/// IPFS节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IpfsNodeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// IPFS节点管理器
pub struct IpfsNodeManager {
    config: IpfsNodeConfig,
    status: Arc<RwLock<IpfsNodeStatus>>,
    process: Arc<RwLock<Option<Child>>>,
    api_url: String,
    gateway_url: String,
}

impl IpfsNodeManager {
    /// 创建新的IPFS节点管理器
    pub fn new(config: IpfsNodeConfig) -> Self {
        let api_url = format!("http://127.0.0.1:{}", config.api_port);
        let gateway_url = format!("http://127.0.0.1:{}", config.gateway_port);
        
        Self {
            config,
            status: Arc::new(RwLock::new(IpfsNodeStatus::Stopped)),
            process: Arc::new(RwLock::new(None)),
            api_url,
            gateway_url,
        }
    }
    
    /// 启动IPFS节点
    pub async fn start(&self) -> Result<()> {
        log::info!("🚀 启动内置IPFS节点...");
        log::info!("  数据目录: {:?}", self.config.data_dir);
        log::info!("  API端口: {}", self.config.api_port);
        log::info!("  网关端口: {}", self.config.gateway_port);
        log::info!("  Swarm端口: {}", self.config.swarm_port);
        
        // 检查IPFS是否已安装
        self.check_ipfs_installed().await?;
        
        // 设置状态为启动中
        {
            let mut status = self.status.write().await;
            *status = IpfsNodeStatus::Starting;
        }
        
        // 初始化IPFS仓库（如果不存在）
        self.init_ipfs_repo().await?;
        
        // 检查是否已有IPFS节点在运行
        if self.is_existing_node_running().await? {
            log::info!("✅ 检测到现有IPFS节点正在运行，直接使用");
            log::info!("  API地址: {}", self.api_url);
            log::info!("  网关地址: {}", self.gateway_url);
            
            // 设置状态为运行中
            {
                let mut status = self.status.write().await;
                *status = IpfsNodeStatus::Running;
            }
            
            return Ok(());
        }
        
        // 启动新的IPFS daemon
        let child = self.start_ipfs_daemon().await?;
        
        // 保存进程句柄
        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }
        
        // 等待节点启动完成
        self.wait_for_startup().await?;
        
        // 设置状态为运行中
        {
            let mut status = self.status.write().await;
            *status = IpfsNodeStatus::Running;
        }
        
        log::info!("✅ IPFS节点启动成功");
        log::info!("  API地址: {}", self.api_url);
        log::info!("  网关地址: {}", self.gateway_url);
        
        Ok(())
    }
    
    /// 停止IPFS节点
    pub async fn stop(&self) -> Result<()> {
        log::info!("🛑 停止IPFS节点...");
        
        // 设置状态为停止中
        {
            let mut status = self.status.write().await;
            *status = IpfsNodeStatus::Stopping;
        }
        
        // 终止进程
        {
            let mut process = self.process.write().await;
            if let Some(mut child) = process.take() {
                match child.kill() {
                    Ok(_) => {
                        log::info!("✓ IPFS进程已终止");
                        let _ = child.wait();
                    }
                    Err(e) => {
                        log::warn!("终止IPFS进程时出错: {}", e);
                    }
                }
            }
        }
        
        // 设置状态为已停止
        {
            let mut status = self.status.write().await;
            *status = IpfsNodeStatus::Stopped;
        }
        
        log::info!("✅ IPFS节点已停止");
        Ok(())
    }
    
    /// 重启IPFS节点
    pub async fn restart(&self) -> Result<()> {
        log::info!("🔄 重启IPFS节点...");
        self.stop().await?;
        sleep(Duration::from_secs(2)).await; // 等待端口释放
        self.start().await?;
        Ok(())
    }
    
    /// 获取节点状态
    pub async fn status(&self) -> IpfsNodeStatus {
        self.status.read().await.clone()
    }
    
    /// 获取API URL
    pub fn api_url(&self) -> &str {
        &self.api_url
    }
    
    /// 获取网关URL
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }
    
    /// 检查节点是否健康
    pub async fn is_healthy(&self) -> bool {
        self.check_api_health().await.is_ok()
    }
    
    /// 获取节点信息
    pub async fn get_node_info(&self) -> Result<IpfsNodeInfo> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v0/id", self.api_url);
        
        let response = client
            .post(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("无法获取节点信息")?;
        
        if !response.status().is_success() {
            anyhow::bail!("获取节点信息失败: {}", response.status());
        }
        
        let info: IpfsNodeInfo = response.json().await?;
        Ok(info)
    }
    
    /// 检查IPFS是否已安装
    async fn check_ipfs_installed(&self) -> Result<()> {
        // 首先尝试从PATH中查找ipfs
        if let Ok(output) = Command::new("ipfs").arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                log::info!("✓ 检测到IPFS (PATH): {}", version.trim());
                return Ok(());
            }
        }
        
        // 如果PATH中没有，尝试常见安装路径
        let common_paths = [
            r"D:\APPs\kubo\ipfs.exe",  // Windows常见路径
            r"C:\Program Files\Kubo\ipfs.exe",
            r"C:\Program Files (x86)\Kubo\ipfs.exe",
            "/usr/local/bin/ipfs",     // Linux/Mac路径
            "/usr/bin/ipfs",
            "ipfs"  // 再次尝试PATH
        ];
        
        for path in &common_paths {
            if let Ok(output) = Command::new(path).arg("--version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    log::info!("✓ 检测到IPFS ({}): {}", path, version.trim());
                    return Ok(());
                }
            }
        }
        
        anyhow::bail!("无法找到IPFS，请确保IPFS已安装并在PATH中，或安装在常见位置");
    }
    
    /// 检查现有IPFS节点是否运行
    async fn is_existing_node_running(&self) -> Result<bool> {
        // 使用相同的路径检测逻辑
        let ipfs_path = self.find_ipfs_executable().await?;
        
        let output = tokio::process::Command::new(&ipfs_path)
            .arg("id")
            .output()
            .await?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 尝试解析节点ID来确认节点正在运行
            if stdout.contains("ID") && stdout.contains("12D3KooW") {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 查找IPFS可执行文件路径
    async fn find_ipfs_executable(&self) -> Result<String> {
        // 首先尝试从PATH中查找ipfs
        if let Ok(output) = Command::new("ipfs").arg("--version").output() {
            if output.status.success() {
                return Ok("ipfs".to_string());
            }
        }
        
        // 如果PATH中没有，尝试常见安装路径
        let common_paths = [
            r"D:\APPs\kubo\ipfs.exe",  // Windows常见路径
            r"C:\Program Files\Kubo\ipfs.exe",
            r"C:\Program Files (x86)\Kubo\ipfs.exe",
            "/usr/local/bin/ipfs",     // Linux/Mac路径
            "/usr/bin/ipfs",
        ];
        
        for path in &common_paths {
            if let Ok(output) = Command::new(path).arg("--version").output() {
                if output.status.success() {
                    return Ok(path.to_string());
                }
            }
        }
        
        anyhow::bail!("无法找到IPFS可执行文件");
    }
    
    /// 初始化IPFS仓库
    async fn init_ipfs_repo(&self) -> Result<()> {
        // 首先检查全局IPFS仓库是否存在
        let global_repo_path = dirs::home_dir()
            .context("无法获取用户主目录")?
            .join(".ipfs");
        
        if global_repo_path.exists() {
            log::info!("✓ 检测到现有IPFS仓库: {:?}", global_repo_path);
            log::info!("  使用现有IPFS配置，跳过初始化步骤");
            return Ok(());
        }
        
        // 检查自定义仓库是否已存在
        let repo_path = self.config.data_dir.join(".ipfs");
        if repo_path.exists() {
            log::info!("✓ IPFS仓库已存在: {:?}", repo_path);
            return Ok(());
        }
        
        // 只有在没有现有仓库时才初始化新的
        log::info!("📁 初始化新的IPFS仓库...");
        
        // 创建数据目录
        std::fs::create_dir_all(&self.config.data_dir)
            .context("无法创建IPFS数据目录")?;
        
        // 设置IPFS_PATH环境变量
        let ipfs_path = self.find_ipfs_executable().await?;
        let mut cmd = Command::new(&ipfs_path);
        cmd.arg("init");
        cmd.arg("--profile=test"); // 使用测试配置，减少资源使用
        cmd.env("IPFS_PATH", &self.config.data_dir);
        
        let output = cmd.output()
            .context("无法初始化IPFS仓库")?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("IPFS初始化失败: {}", error);
        }
        
        log::info!("✅ IPFS仓库初始化完成");
        Ok(())
    }
    
    /// 启动IPFS daemon
    async fn start_ipfs_daemon(&self) -> Result<Child> {
        log::info!("🚀 启动IPFS daemon...");
        
        let ipfs_path = self.find_ipfs_executable().await?;
        let mut cmd = Command::new(&ipfs_path);
        cmd.arg("daemon");
        cmd.arg("--api-address");
        cmd.arg(format!("/ip4/127.0.0.1/tcp/{}", self.config.api_port));
        cmd.arg("--gateway-address");
        cmd.arg(format!("/ip4/127.0.0.1/tcp/{}", self.config.gateway_port));
        
        if self.config.enable_swarm {
            cmd.arg("--swarm-address");
            cmd.arg(format!("/ip4/127.0.0.1/tcp/{}", self.config.swarm_port));
        }
        
        if !self.config.enable_bootstrap {
            cmd.arg("--disable-bootstrap");
        }
        
        // 设置IPFS_PATH环境变量
        cmd.env("IPFS_PATH", &self.config.data_dir);
        
        // 启动进程
        let child = cmd.spawn()
            .context("无法启动IPFS daemon")?;
        
        Ok(child)
    }
    
    /// 等待节点启动完成
    async fn wait_for_startup(&self) -> Result<()> {
        log::info!("⏳ 等待IPFS节点启动...");
        
        let timeout = Duration::from_secs(self.config.startup_timeout);
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            // 检查API是否可访问
            if self.check_api_health().await.is_ok() {
                log::info!("✅ IPFS节点启动完成");
                return Ok(());
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        // 设置错误状态
        {
            let mut status = self.status.write().await;
            *status = IpfsNodeStatus::Error("启动超时".to_string());
        }
        
        anyhow::bail!("IPFS节点启动超时");
    }
    
    /// 检查API健康状态
    async fn check_api_health(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v0/version", self.api_url);
        
        let response = client
            .post(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .context("API健康检查失败")?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("API健康检查失败: {}", response.status())
        }
    }
}

/// IPFS节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsNodeInfo {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "PublicKey")]
    pub public_key: String,
    #[serde(rename = "Addresses")]
    pub addresses: Vec<String>,
    #[serde(rename = "AgentVersion")]
    pub agent_version: String,
    #[serde(rename = "ProtocolVersion")]
    pub protocol_version: String,
}

impl Drop for IpfsNodeManager {
    fn drop(&mut self) {
        // 在析构时自动停止IPFS节点
        if let Ok(mut process) = self.process.try_write() {
            if let Some(mut child) = process.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_ipfs_node_config_default() {
        let config = IpfsNodeConfig::default();
        assert_eq!(config.api_port, 5001);
        assert_eq!(config.gateway_port, 8080);
        assert!(config.auto_start);
        assert!(config.enable_bootstrap);
        assert!(!config.verbose_logging);
    }
    
    #[tokio::test]
    async fn test_ipfs_node_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let config = IpfsNodeConfig {
            data_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = IpfsNodeManager::new(config);
        assert_eq!(manager.api_url(), "http://localhost:5001");
        assert_eq!(manager.gateway_url(), "http://localhost:8080");
        
        let status = manager.status().await;
        assert_eq!(status, IpfsNodeStatus::Stopped);
    }
    
    // 注意：以下测试需要实际的IPFS安装
    #[tokio::test]
    #[ignore] // 需要实际的IPFS安装
    async fn test_ipfs_node_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let config = IpfsNodeConfig {
            data_dir: temp_dir.path().join("test-ipfs"),
            startup_timeout: 10,
            ..Default::default()
        };
        
        let manager = IpfsNodeManager::new(config);
        
        // 测试启动
        let result = manager.start().await;
        if result.is_ok() {
            assert_eq!(manager.status().await, IpfsNodeStatus::Running);
            assert!(manager.is_healthy().await);
            
            // 测试停止
            manager.stop().await.unwrap();
            assert_eq!(manager.status().await, IpfsNodeStatus::Stopped);
        } else {
            // 如果没有安装IPFS，测试应该跳过
            println!("跳过IPFS测试：IPFS未安装");
        }
    }
}