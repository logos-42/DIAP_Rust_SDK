// DIAP Rust SDK - 配置管理模块
// Decentralized Intelligent Agent Protocol
// 负责加载、保存和管理SDK配置

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SDK配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIAPConfig {
    /// 智能体配置
    pub agent: AgentConfig,

    /// IPFS配置
    pub ipfs: IpfsConfig,

    /// IPNS配置
    pub ipns: IpnsConfig,

    /// 缓存配置
    pub cache: CacheConfig,

    /// 日志配置
    pub logging: LoggingConfig,
}

/// 智能体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 智能体名称
    pub name: String,

    /// 私钥文件路径
    pub private_key_path: PathBuf,

    /// 是否自动生成密钥（如果文件不存在）
    #[serde(default = "default_true")]
    pub auto_generate_key: bool,
}

/// IPFS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfig {
    /// AWS IPFS节点API地址（优先）
    pub aws_api_url: Option<String>,

    /// AWS IPFS网关地址
    pub aws_gateway_url: Option<String>,

    /// Pinata API密钥（备用）
    pub pinata_api_key: Option<String>,

    /// Pinata API密钥
    pub pinata_api_secret: Option<String>,

    /// 超时时间（秒）
    #[serde(default = "default_ipfs_timeout")]
    pub timeout_seconds: u64,
}

/// IPNS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpnsConfig {
    /// 是否使用w3name（优先）
    #[serde(default = "default_true")]
    pub use_w3name: bool,

    /// 是否使用IPFS节点（备用）
    #[serde(default = "default_true")]
    pub use_ipfs_node: bool,

    /// IPNS记录有效期（天）
    #[serde(default = "default_ipns_validity_days")]
    pub validity_days: u64,
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 是否启用缓存
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 缓存TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,

    /// 最大缓存条目数
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: usize,

    /// 缓存目录
    pub cache_dir: Option<PathBuf>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,
}

// 默认值函数
fn default_true() -> bool {
    true
}
fn default_ipfs_timeout() -> u64 {
    30
}
fn default_ipns_validity_days() -> u64 {
    365
}
fn default_cache_ttl() -> u64 {
    21600
} // 6小时
fn default_cache_max_entries() -> usize {
    1000
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Default for DIAPConfig {
    fn default() -> Self {
        let dirs = ProjectDirs::from("com", "diap", "diap-rs-sdk").expect("无法获取项目目录");

        Self {
            agent: AgentConfig {
                name: "DIAP Agent".to_string(),
                private_key_path: dirs.data_dir().join("keys/agent.key"),
                auto_generate_key: true,
            },
            ipfs: IpfsConfig {
                aws_api_url: None,
                aws_gateway_url: None,
                pinata_api_key: None,
                pinata_api_secret: None,
                timeout_seconds: 30,
            },
            ipns: IpnsConfig {
                use_w3name: true,
                use_ipfs_node: true,
                validity_days: 365,
            },
            cache: CacheConfig {
                enabled: true,
                ttl_seconds: 21600,
                max_entries: 1000,
                cache_dir: Some(dirs.cache_dir().to_path_buf()),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }
}

impl DIAPConfig {
    /// 从文件加载配置
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件: {:?}", path))?;

        let config: DIAPConfig =
            toml::from_str(&content).with_context(|| format!("无法解析配置文件: {:?}", path))?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建配置目录: {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self).context("无法序列化配置")?;

        std::fs::write(path, content).with_context(|| format!("无法写入配置文件: {:?}", path))?;

        Ok(())
    }

    /// 获取默认配置文件路径
    pub fn default_config_path() -> PathBuf {
        let dirs = ProjectDirs::from("com", "diap", "diap-rs-sdk").expect("无法获取项目目录");
        dirs.config_dir().join("config.toml")
    }

    /// 加载配置（优先从文件，否则使用默认值）
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            log::info!("从文件加载配置: {:?}", config_path);
            Self::from_file(&config_path)
        } else {
            log::info!("使用默认配置");
            let config = Self::default();

            // 尝试保存默认配置
            if let Err(e) = config.save_to_file(&config_path) {
                log::warn!("无法保存默认配置: {}", e);
            } else {
                log::info!("已保存默认配置到: {:?}", config_path);
            }

            Ok(config)
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证IPFS配置
        if self.ipfs.aws_api_url.is_none() && self.ipfs.pinata_api_key.is_none() {
            anyhow::bail!("必须配置AWS IPFS节点或Pinata");
        }

        // 验证IPNS配置
        if !self.ipns.use_w3name && !self.ipns.use_ipfs_node {
            anyhow::bail!("必须至少启用一种IPNS发布方式");
        }

        // 验证日志级别
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            anyhow::bail!("无效的日志级别: {}", self.logging.level);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DIAPConfig::default();
        assert_eq!(config.agent.name, "DIAP Agent");
        assert!(config.cache.enabled);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_serialization() {
        let config = DIAPConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: DIAPConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.agent.name, deserialized.agent.name);
    }
}
