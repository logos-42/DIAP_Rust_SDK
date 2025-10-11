// DIAP Rust SDK - 批量上传和自动更新模块
// Decentralized Intelligent Agent Protocol
// 支持批量上传多个DID文档和自动定时更新

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use crate::key_manager::KeyPair;
use crate::did_builder::{DIDBuilder, DIDPublishResult};

/// 批量上传结果
#[derive(Debug, Clone)]
pub struct BatchUploadResult {
    /// 成功的数量
    pub success_count: usize,
    
    /// 失败的数量
    pub failure_count: usize,
    
    /// 详细结果
    pub results: Vec<BatchItemResult>,
    
    /// 总耗时（秒）
    pub total_duration: f64,
}

/// 单个批量项结果
#[derive(Debug, Clone)]
pub struct BatchItemResult {
    /// 智能体名称
    pub agent_name: String,
    
    /// 是否成功
    pub success: bool,
    
    /// DID（如果成功）
    pub did: Option<String>,
    
    /// CID（如果成功）
    pub cid: Option<String>,
    
    /// 错误信息（如果失败）
    pub error: Option<String>,
    
    /// 耗时（秒）
    pub duration: f64,
}

/// 批量上传器
pub struct BatchUploader {
    /// DID构建器
    did_builder: Arc<DIDBuilder>,
    
    /// 最大并发数
    max_concurrent: usize,
}

impl BatchUploader {
    /// 创建新的批量上传器
    pub fn new(did_builder: DIDBuilder, max_concurrent: usize) -> Self {
        Self {
            did_builder: Arc::new(did_builder),
            max_concurrent,
        }
    }
    
    /// 批量上传DID文档
    pub async fn batch_upload(
        &self,
        items: Vec<(String, KeyPair)>,  // (agent_name, keypair)
    ) -> Result<BatchUploadResult> {
        let start_time = std::time::Instant::now();
        let total_count = items.len();
        
        log::info!("开始批量上传 {} 个DID文档", total_count);
        
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;
        
        // 使用信号量控制并发
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrent));
        let mut tasks = Vec::new();
        
        for (agent_name, keypair) in items {
            let semaphore = semaphore.clone();
            let did_builder = self.did_builder.clone();
            
            let task = tokio::spawn(async move {
                // 获取信号量许可
                let _permit = semaphore.acquire().await.unwrap();
                
                let item_start = std::time::Instant::now();
                
                // 执行上传
                let result = did_builder.create_and_publish(&keypair).await;
                
                let duration = item_start.elapsed().as_secs_f64();
                
                match result {
                    Ok(publish_result) => {
                        log::info!("✓ {} 上传成功", agent_name);
                        BatchItemResult {
                            agent_name,
                            success: true,
                            did: Some(publish_result.did),
                            cid: Some(publish_result.current_cid),
                            error: None,
                            duration,
                        }
                    }
                    Err(e) => {
                        log::error!("✗ {} 上传失败: {}", agent_name, e);
                        BatchItemResult {
                            agent_name,
                            success: false,
                            did: None,
                            cid: None,
                            error: Some(e.to_string()),
                            duration,
                        }
                    }
                }
            });
            
            tasks.push(task);
        }
        
        // 等待所有任务完成
        for task in tasks {
            let result = task.await.unwrap();
            if result.success {
                success_count += 1;
            } else {
                failure_count += 1;
            }
            results.push(result);
        }
        
        let total_duration = start_time.elapsed().as_secs_f64();
        
        log::info!("批量上传完成: 成功 {}/{}, 耗时 {:.2}秒", 
                   success_count, total_count, total_duration);
        
        Ok(BatchUploadResult {
            success_count,
            failure_count,
            results,
            total_duration,
        })
    }
}

/// 自动更新管理器
pub struct AutoUpdateManager {
    /// DID构建器
    did_builder: Arc<DIDBuilder>,
    
    /// 密钥对
    keypair: Arc<KeyPair>,
    
    /// 当前状态
    state: Arc<RwLock<UpdateState>>,
    
    /// 更新间隔（小时）
    update_interval_hours: u64,
    
    /// 是否运行
    is_running: Arc<RwLock<bool>>,
}

/// 更新状态
#[derive(Debug, Clone)]
pub struct UpdateState {
    /// 当前序列号
    current_sequence: u64,
    
    /// 当前CID
    current_cid: String,
    
    /// 上次更新时间
    last_update: String,
    
    /// 更新次数
    update_count: u64,
}

impl AutoUpdateManager {
    /// 创建新的自动更新管理器
    pub fn new(
        did_builder: DIDBuilder,
        keypair: KeyPair,
        initial_sequence: u64,
        initial_cid: String,
        update_interval_hours: u64,
    ) -> Self {
        let state = UpdateState {
            current_sequence: initial_sequence,
            current_cid: initial_cid,
            last_update: chrono::Utc::now().to_rfc3339(),
            update_count: 0,
        };
        
        Self {
            did_builder: Arc::new(did_builder),
            keypair: Arc::new(keypair),
            state: Arc::new(RwLock::new(state)),
            update_interval_hours,
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 启动自动更新
    pub async fn start(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            log::warn!("自动更新已在运行");
            return;
        }
        *is_running = true;
        drop(is_running);
        
        log::info!("启动自动更新管理器，间隔: {}小时", self.update_interval_hours);
        
        let did_builder = self.did_builder.clone();
        let keypair = self.keypair.clone();
        let state = self.state.clone();
        let is_running = self.is_running.clone();
        let interval_hours = self.update_interval_hours;
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_hours * 3600));
            
            loop {
                ticker.tick().await;
                
                // 检查是否还在运行
                let running = *is_running.read().await;
                if !running {
                    log::info!("自动更新管理器已停止");
                    break;
                }
                
                log::info!("执行定时更新...");
                
                // 读取当前状态
                let current_sequence = {
                    let state_guard = state.read().await;
                    state_guard.current_sequence
                };
                
                // 执行更新（刷新IPNS有效期）
                match did_builder.update_did_document(
                    &keypair,
                    current_sequence,
                    |_did_doc| {
                        // 不修改内容，只是刷新
                    },
                ).await {
                    Ok(result) => {
                        // 更新状态
                        let mut state_guard = state.write().await;
                        state_guard.current_sequence = result.sequence;
                        state_guard.current_cid = result.current_cid.clone();
                        state_guard.last_update = chrono::Utc::now().to_rfc3339();
                        state_guard.update_count += 1;
                        
                        log::info!("✓ 自动更新成功, 序列号: {}", result.sequence);
                    }
                    Err(e) => {
                        log::error!("✗ 自动更新失败: {}", e);
                    }
                }
            }
        });
    }
    
    /// 停止自动更新
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        log::info!("停止自动更新管理器");
    }
    
    /// 获取当前状态
    pub async fn get_state(&self) -> UpdateState {
        let state = self.state.read().await;
        state.clone()
    }
    
    /// 手动触发更新
    pub async fn trigger_update(&self) -> Result<DIDPublishResult> {
        log::info!("手动触发更新");
        
        let current_sequence = {
            let state = self.state.read().await;
            state.current_sequence
        };
        
        let result = self.did_builder.update_did_document(
            &self.keypair,
            current_sequence,
            |_did_doc| {
                // 不修改内容
            },
        ).await?;
        
        // 更新状态
        let mut state = self.state.write().await;
        state.current_sequence = result.sequence;
        state.current_cid = result.current_cid.clone();
        state.last_update = chrono::Utc::now().to_rfc3339();
        state.update_count += 1;
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipfs_client::IpfsClient;
    use crate::ipns_publisher::IpnsPublisher;
    
    #[test]
    fn test_batch_uploader_creation() {
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let ipns_publisher = IpnsPublisher::new(true, false, None, 365);
        let did_builder = DIDBuilder::new(
            "Test".to_string(),
            ipfs_client,
            ipns_publisher,
        );
        
        let uploader = BatchUploader::new(did_builder, 10);
        assert_eq!(uploader.max_concurrent, 10);
    }
    
    #[tokio::test]
    async fn test_auto_update_manager() {
        let ipfs_client = IpfsClient::new(None, None, None, None, 30);
        let ipns_publisher = IpnsPublisher::new(true, false, None, 365);
        let did_builder = DIDBuilder::new(
            "Test".to_string(),
            ipfs_client,
            ipns_publisher,
        );
        let keypair = KeyPair::generate().unwrap();
        
        let manager = AutoUpdateManager::new(
            did_builder,
            keypair,
            1,
            "QmTest...".to_string(),
            24,
        );
        
        let state = manager.get_state().await;
        assert_eq!(state.current_sequence, 1);
    }
}
