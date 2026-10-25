// DIAP Rust SDK - DID文档缓存
// 减少IPFS请求，提高验证性能

use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use serde::{Deserialize, Serialize};
use crate::did_builder::DIDDocument;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// DID文档
    pub document: DIDDocument,
    
    /// CID
    pub cid: String,
    
    /// 缓存时间
    pub cached_at: u64,
    
    /// 过期时间
    pub expires_at: u64,
    
    /// 访问次数
    pub hit_count: u64,
}

/// DID文档缓存管理器
#[derive(Clone)]
pub struct DIDCache {
    /// CID -> DIDDocument 缓存
    cache: Arc<DashMap<String, CacheEntry>>,
    
    /// 缓存有效期（秒）
    ttl: u64,
    
    /// 最大缓存条目数
    max_entries: usize,
}

impl DIDCache {
    /// 创建新的DID缓存
    /// 
    /// # 参数
    /// * `ttl` - 缓存有效期（秒），默认3600秒（1小时）
    /// * `max_entries` - 最大缓存条目数，默认1000
    pub fn new(ttl: Option<u64>, max_entries: Option<usize>) -> Self {
        let ttl_seconds = ttl.unwrap_or(3600);
        let max = max_entries.unwrap_or(1000);
        
        let cache = Self {
            cache: Arc::new(DashMap::new()),
            ttl: ttl_seconds,
            max_entries: max,
        };
        
        // 启动后台清理任务
        cache.start_cleanup_task();
        
        log::info!("💾 DID文档缓存已创建");
        log::info!("  TTL: {}秒", ttl_seconds);
        log::info!("  最大条目: {}", max);
        
        cache
    }
    
    /// 获取DID文档
    pub fn get(&self, cid: &str) -> Option<DIDDocument> {
        if let Some(mut entry) = self.cache.get_mut(cid) {
            let now = Self::current_timestamp();
            
            // 检查是否过期
            if entry.expires_at < now {
                drop(entry);
                self.cache.remove(cid);
                log::debug!("缓存已过期: {}", cid);
                return None;
            }
            
            // 增加命中次数
            entry.hit_count += 1;
            let doc = entry.document.clone();
            
            log::debug!("✓ 缓存命中: {} (命中次数: {})", cid, entry.hit_count);
            return Some(doc);
        }
        
        log::debug!("缓存未命中: {}", cid);
        None
    }
    
    /// 存储DID文档
    pub fn put(&self, cid: String, document: DIDDocument) -> Result<()> {
        // 检查缓存大小
        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }
        
        let now = Self::current_timestamp();
        let entry = CacheEntry {
            document,
            cid: cid.clone(),
            cached_at: now,
            expires_at: now + self.ttl,
            hit_count: 0,
        };
        
        self.cache.insert(cid.clone(), entry);
        log::debug!("✓ 已缓存DID文档: {}", cid);
        
        Ok(())
    }
    
    /// 移除缓存条目
    pub fn remove(&self, cid: &str) -> Option<DIDDocument> {
        self.cache.remove(cid).map(|(_, entry)| {
            log::debug!("移除缓存: {}", cid);
            entry.document
        })
    }
    
    /// 清空缓存
    pub fn clear(&self) {
        let count = self.cache.len();
        self.cache.clear();
        log::info!("🧹 清空缓存: {} 个条目", count);
    }
    
    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let mut total_hits = 0u64;
        let mut expired = 0usize;
        let now = Self::current_timestamp();
        
        for entry in self.cache.iter() {
            total_hits += entry.hit_count;
            if entry.expires_at < now {
                expired += 1;
            }
        }
        
        CacheStats {
            total_entries: self.cache.len(),
            expired_entries: expired,
            total_hits,
            max_entries: self.max_entries,
            ttl: self.ttl,
        }
    }
    
    /// 清理过期条目
    pub fn cleanup_expired(&self) -> usize {
        let now = Self::current_timestamp();
        let mut removed = 0;
        
        self.cache.retain(|_, entry| {
            if entry.expires_at < now {
                removed += 1;
                false
            } else {
                true
            }
        });
        
        if removed > 0 {
            log::debug!("🧹 清理了 {} 个过期缓存", removed);
        }
        
        removed
    }
    
    /// 驱逐最少使用的条目（LRU）
    fn evict_lru(&self) {
        // 找到命中次数最少的条目
        let mut min_hits = u64::MAX;
        let mut evict_cid: Option<String> = None;
        
        for entry in self.cache.iter() {
            if entry.hit_count < min_hits {
                min_hits = entry.hit_count;
                evict_cid = Some(entry.cid.clone());
            }
        }
        
        if let Some(cid) = evict_cid {
            self.cache.remove(&cid);
            log::debug!("驱逐LRU缓存: {} (命中次数: {})", cid, min_hits);
        }
    }
    
    /// 获取当前时间戳
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    /// 启动后台清理任务
    fn start_cleanup_task(&self) {
        let cache = self.cache.clone();
        let ttl = self.ttl;
        
        tokio::spawn(async move {
            // 每隔TTL/4清理一次
            let interval = Duration::from_secs(ttl / 4);
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let mut removed = 0;
                cache.retain(|_, entry| {
                    if entry.expires_at < now {
                        removed += 1;
                        false
                    } else {
                        true
                    }
                });
                
                if removed > 0 {
                    log::debug!("🧹 后台清理了 {} 个过期DID缓存", removed);
                }
            }
        });
    }
}

impl Default for DIDCache {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_hits: u64,
    pub max_entries: usize,
    pub ttl: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::did_builder::VerificationMethod;
    
    fn create_test_document(did: &str) -> DIDDocument {
        DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: did.to_string(),
            verification_method: vec![VerificationMethod {
                id: format!("{}#key-1", did),
                vm_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.to_string(),
                public_key_multibase: "z6MkTest".to_string(),
            }],
            authentication: vec![format!("{}#key-1", did)],
            service: None,
            created: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    #[tokio::test]
    async fn test_cache_put_and_get() {
        let cache = DIDCache::new(Some(300), Some(100));
        let cid = "QmTest123";
        let doc = create_test_document("did:key:z6MkTest");
        
        // 存储
        cache.put(cid.to_string(), doc.clone()).unwrap();
        
        // 获取
        let retrieved = cache.get(cid);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, doc.id);
    }
    
    #[tokio::test]
    async fn test_cache_miss() {
        let cache = DIDCache::new(Some(300), Some(100));
        let result = cache.get("QmNonExistent");
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_remove() {
        let cache = DIDCache::new(Some(300), Some(100));
        let cid = "QmTest456";
        let doc = create_test_document("did:key:z6MkTest2");
        
        cache.put(cid.to_string(), doc.clone()).unwrap();
        assert!(cache.get(cid).is_some());
        
        cache.remove(cid);
        assert!(cache.get(cid).is_none());
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = DIDCache::new(Some(1), Some(100));  // 1秒TTL
        let cid = "QmTest789";
        let doc = create_test_document("did:key:z6MkTest3");
        
        cache.put(cid.to_string(), doc).unwrap();
        assert!(cache.get(cid).is_some());
        
        // 等待过期
        std::thread::sleep(Duration::from_secs(2));
        
        assert!(cache.get(cid).is_none());
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let cache = DIDCache::new(Some(300), Some(100));
        
        for i in 0..5 {
            let cid = format!("QmTest{}", i);
            let doc = create_test_document(&format!("did:key:test{}", i));
            cache.put(cid, doc).unwrap();
        }
        
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.max_entries, 100);
    }
    
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = DIDCache::new(Some(300), Some(3));  // 只能存3个
        
        // 添加3个文档
        for i in 0..3 {
            let cid = format!("QmTest{}", i);
            let doc = create_test_document(&format!("did:key:test{}", i));
            cache.put(cid, doc).unwrap();
        }
        
        // 访问前两个，增加命中次数
        cache.get("QmTest0");
        cache.get("QmTest0");
        cache.get("QmTest1");
        
        // 添加第4个，应该驱逐QmTest2（命中次数最少）
        let doc = create_test_document("did:key:test3");
        cache.put("QmTest3".to_string(), doc).unwrap();
        
        assert!(cache.get("QmTest0").is_some());
        assert!(cache.get("QmTest1").is_some());
        assert!(cache.get("QmTest2").is_none());  // 被驱逐
        assert!(cache.get("QmTest3").is_some());
    }
}

