// DIAP Rust SDK - Nonce管理器
// 防止重放攻击，跟踪已使用的nonce

use anyhow::{Context, Result};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use serde::{Deserialize, Serialize};

/// Nonce记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceRecord {
    /// nonce值
    pub nonce: String,
    
    /// 使用时间戳
    pub used_at: u64,
    
    /// 关联的DID
    pub did: String,
    
    /// 过期时间戳
    pub expires_at: u64,
}

/// Nonce管理器
/// 使用DashMap实现线程安全的高性能nonce追踪
#[derive(Clone)]
pub struct NonceManager {
    /// nonce存储 (nonce -> NonceRecord)
    nonces: Arc<DashMap<String, NonceRecord>>,
    
    /// nonce有效期（秒）
    validity_duration: u64,
    
    /// 清理间隔（秒）
    cleanup_interval: u64,
}

impl NonceManager {
    /// 创建新的Nonce管理器
    /// 
    /// # 参数
    /// * `validity_duration` - nonce有效期（秒），默认300秒（5分钟）
    /// * `cleanup_interval` - 清理过期nonce的间隔（秒），默认60秒
    pub fn new(validity_duration: Option<u64>, cleanup_interval: Option<u64>) -> Self {
        let validity = validity_duration.unwrap_or(300);
        let cleanup = cleanup_interval.unwrap_or(60);
        
        let manager = Self {
            nonces: Arc::new(DashMap::new()),
            validity_duration: validity,
            cleanup_interval: cleanup,
        };
        
        // 启动后台清理任务
        manager.start_cleanup_task();
        
        log::info!("🔐 Nonce管理器已创建");
        log::info!("  有效期: {}秒", validity);
        log::info!("  清理间隔: {}秒", cleanup);
        
        manager
    }
    
    /// 生成新的nonce
    /// 格式: timestamp:uuid:random
    pub fn generate_nonce() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let uuid = uuid::Uuid::new_v4();
        let random = rand::random::<u64>();
        
        format!("{}:{}:{:x}", timestamp, uuid, random)
    }
    
    /// 验证并记录nonce
    /// 
    /// # 返回
    /// * `Ok(true)` - nonce有效且未被使用
    /// * `Ok(false)` - nonce已被使用（重放攻击）
    /// * `Err` - nonce格式错误或已过期
    pub fn verify_and_record(&self, nonce: &str, did: &str) -> Result<bool> {
        // 1. 解析nonce
        let parts: Vec<&str> = nonce.split(':').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Nonce格式错误"));
        }
        
        let timestamp: u64 = parts[0].parse()
            .context("无法解析时间戳")?;
        
        // 2. 检查时间戳是否在有效期内
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if timestamp > now {
            return Err(anyhow::anyhow!("Nonce时间戳在未来"));
        }
        
        if now - timestamp > self.validity_duration {
            return Err(anyhow::anyhow!(
                "Nonce已过期（超过{}秒）",
                self.validity_duration
            ));
        }
        
        // 3. 检查是否已被使用
        if self.nonces.contains_key(nonce) {
            log::warn!("检测到重放攻击！Nonce已被使用: {}", nonce);
            return Ok(false);
        }
        
        // 4. 记录nonce
        let record = NonceRecord {
            nonce: nonce.to_string(),
            used_at: now,
            did: did.to_string(),
            expires_at: now + self.validity_duration,
        };
        
        self.nonces.insert(nonce.to_string(), record);
        
        log::debug!("✓ Nonce验证通过并已记录: {}", nonce);
        Ok(true)
    }
    
    /// 检查nonce是否已被使用
    pub fn is_used(&self, nonce: &str) -> bool {
        self.nonces.contains_key(nonce)
    }
    
    /// 获取nonce记录
    pub fn get_record(&self, nonce: &str) -> Option<NonceRecord> {
        self.nonces.get(nonce).map(|r| r.clone())
    }
    
    /// 清理过期的nonce
    pub fn cleanup_expired(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut removed = 0;
        
        self.nonces.retain(|_, record| {
            if record.expires_at < now {
                removed += 1;
                false  // 移除
            } else {
                true   // 保留
            }
        });
        
        if removed > 0 {
            log::info!("🧹 清理了 {} 个过期nonce", removed);
        }
        
        removed
    }
    
    /// 获取当前nonce数量
    pub fn count(&self) -> usize {
        self.nonces.len()
    }
    
    /// 清空所有nonce（测试用）
    pub fn clear(&self) {
        self.nonces.clear();
        log::warn!("⚠️ 所有nonce已清空");
    }
    
    /// 启动后台清理任务
    fn start_cleanup_task(&self) {
        let nonces = self.nonces.clone();
        let interval = self.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(interval));
            
            loop {
                interval_timer.tick().await;
                
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let mut removed = 0;
                nonces.retain(|_, record| {
                    if record.expires_at < now {
                        removed += 1;
                        false
                    } else {
                        true
                    }
                });
                
                if removed > 0 {
                    log::debug!("🧹 后台清理了 {} 个过期nonce", removed);
                }
            }
        });
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_generate_nonce() {
        let nonce1 = NonceManager::generate_nonce();
        let nonce2 = NonceManager::generate_nonce();
        
        assert_ne!(nonce1, nonce2);
        assert!(nonce1.contains(':'));
        
        println!("生成的nonce: {}", nonce1);
    }
    
    #[tokio::test]
    async fn test_verify_and_record() {
        let manager = NonceManager::new(Some(300), Some(60));
        let nonce = NonceManager::generate_nonce();
        let did = "did:key:z6MkTest";
        
        // 第一次使用应该成功
        let result = manager.verify_and_record(&nonce, did);
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // 第二次使用应该失败（重放攻击）
        let result = manager.verify_and_record(&nonce, did);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
    
    #[tokio::test]
    async fn test_expired_nonce() {
        let manager = NonceManager::new(Some(1), Some(60));  // 1秒有效期
        
        // 创建一个过去的nonce
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 10;  // 10秒前
        
        let old_nonce = format!("{}:test:abc", old_timestamp);
        
        let result = manager.verify_and_record(&old_nonce, "did:key:test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("过期"));
    }
    
    #[tokio::test]
    async fn test_cleanup() {
        let manager = NonceManager::new(Some(1), Some(60));
        
        // 添加一些nonce
        for i in 0..5 {
            let nonce = format!("{}:test:{}", 
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                i
            );
            manager.verify_and_record(&nonce, "did:key:test").ok();
        }
        
        assert_eq!(manager.count(), 5);
        
        // 等待过期
        std::thread::sleep(Duration::from_secs(2));
        
        // 清理
        let removed = manager.cleanup_expired();
        assert_eq!(removed, 5);
        assert_eq!(manager.count(), 0);
    }
    
    #[tokio::test]
    async fn test_invalid_nonce_format() {
        let manager = NonceManager::new(Some(300), Some(60));
        
        let result = manager.verify_and_record("invalid", "did:key:test");
        assert!(result.is_err());
    }
}

