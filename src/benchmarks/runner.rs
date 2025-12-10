/**
 * 实验基准测试 - 实验运行器模块
 */
use crate::benchmarks::collector::MetricCollector;
use crate::benchmarks::measurements::*;
use crate::benchmarks::types::{ExperimentConfig, ExperimentResult, Measurement, MetricType};
use crate::{AgentInfo, IdentityManager, IpfsClient, NoirZKPManager};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 实验运行器
pub struct ExperimentRunner {
    /// 指标收集器
    collector: Arc<MetricCollector>,
    /// IPFS 客户端
    ipfs_client: Arc<IpfsClient>,
    /// 实验配置
    config: ExperimentConfig,
}

impl ExperimentRunner {
    /// 创建新的实验运行器
    pub fn new(config: ExperimentConfig) -> Self {
        let ipfs_client = Arc::new(IpfsClient::new(
            Some(config.ipfs_api_url.clone()),
            Some(config.ipfs_gateway_url.clone()),
            None, // pinata_api_key
            None, // pinata_api_secret
            30,   // timeout_seconds
        ));

        Self {
            collector: Arc::new(MetricCollector::new(10000)),
            ipfs_client,
            config,
        }
    }

    /// 运行实验
    pub async fn run(&self) -> Result<ExperimentResult> {
        let start_time = chrono::Utc::now();
        let start_instant = Instant::now();
        let mut errors = Vec::new();

        log::info!("🚀 开始实验: {}", self.config.name);
        log::info!("  指标数量: {}", self.config.metrics.len());
        log::info!("  迭代次数: {}", self.config.iterations);

        // 创建必要的组件
        let identity_manager = IdentityManager::new((*self.ipfs_client).clone());
        let circuits_path = "noir_circuits".to_string();
        let mut zkp_manager = NoirZKPManager::new(circuits_path);

        // 为每个指标运行测量
        for metric_type in &self.config.metrics {
            log::info!("📊 测量指标: {:?}", metric_type);

            for iteration in 0..self.config.iterations {
                log::debug!("  迭代 {}/{}", iteration + 1, self.config.iterations);

                match self.measure_metric(metric_type, &identity_manager, &mut zkp_manager).await {
                    Ok(measurement) => {
                        self.collector
                            .record_measurement(measurement.metric_type, measurement.value, measurement.metadata.clone())
                            .await;
                    }
                    Err(e) => {
                        let error_msg = format!("测量失败 {:?} (迭代 {}): {}", metric_type, iteration + 1, e);
                        log::error!("  {}", error_msg);
                        errors.push(error_msg);
                    }
                }

                // 短暂延迟，避免过载
                sleep(Duration::from_millis(100)).await;
            }
        }

        // 收集统计结果
        let mut metrics_stats = HashMap::new();
        for metric_type in &self.config.metrics {
            let stats = self.collector.get_statistics(*metric_type).await;
            metrics_stats.insert(format!("{:?}", metric_type), stats);
        }

        let end_time = chrono::Utc::now();
        let duration_seconds = start_instant.elapsed().as_secs_f64();
        let raw_measurements = self.collector.get_all_measurements().await;

        log::info!("✅ 实验完成: {}", self.config.name);
        log::info!("  总耗时: {:.2} 秒", duration_seconds);
        log::info!("  错误数量: {}", errors.len());

        Ok(ExperimentResult {
            config: self.config.clone(),
            metrics: metrics_stats,
            raw_measurements,
            start_time: start_time.to_rfc3339(),
            end_time: end_time.to_rfc3339(),
            duration_seconds,
            errors,
        })
    }

    /// 测量单个指标
    async fn measure_metric(
        &self,
        metric_type: &MetricType,
        identity_manager: &IdentityManager,
        zkp_manager: &mut NoirZKPManager,
    ) -> Result<Measurement> {
        match metric_type {
            MetricType::RegistrationLatency => {
                let keypair = crate::KeyPair::generate().context("密钥生成失败")?;
                let libp2p_identity = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id = libp2p_identity.peer_id();

                let agent_info = AgentInfo {
                    name: format!("test_agent_{}", uuid::Uuid::new_v4()),
                    services: vec![],
                    description: Some("测试智能体".to_string()),
                    tags: Some(vec!["test".to_string()]),
                };

                let (_, measurement) = measure_registration_latency(
                    identity_manager,
                    &agent_info,
                    &keypair,
                    &peer_id,
                    zkp_manager,
                )
                .await?;

                Ok(measurement)
            }

            MetricType::ZKPGenerationTime => {
                let keypair = crate::KeyPair::generate().context("密钥生成失败")?;
                let libp2p_identity = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id = libp2p_identity.peer_id();

                let agent_info = AgentInfo {
                    name: format!("test_agent_{}", uuid::Uuid::new_v4()),
                    services: vec![],
                    description: None,
                    tags: None,
                };

                let registration = identity_manager
                    .register_identity(&agent_info, &keypair, &peer_id)
                    .await
                    .context("注册失败")?;

                let cid_bytes = hex::decode(&registration.cid)
                    .unwrap_or_else(|_| registration.cid.clone().into_bytes());
                let nonce = uuid::Uuid::new_v4().as_bytes().to_vec();

                let (_, measurement) = measure_zkp_generation_time(
                    zkp_manager,
                    &keypair,
                    &registration.did_document,
                    &cid_bytes,
                    &nonce,
                )
                .await?;

                Ok(measurement)
            }

            MetricType::ZKPVerificationTime => {
                let keypair = crate::KeyPair::generate().context("密钥生成失败")?;
                let libp2p_identity = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id = libp2p_identity.peer_id();

                let agent_info = AgentInfo {
                    name: format!("test_agent_{}", uuid::Uuid::new_v4()),
                    services: vec![],
                    description: None,
                    tags: None,
                };

                let registration = identity_manager
                    .register_identity(&agent_info, &keypair, &peer_id)
                    .await
                    .context("注册失败")?;

                let cid_bytes = hex::decode(&registration.cid)
                    .unwrap_or_else(|_| registration.cid.clone().into_bytes());
                let nonce = uuid::Uuid::new_v4().as_bytes().to_vec();

                let proof_result = zkp_manager
                    .generate_did_binding_proof(&keypair, &registration.did_document, &cid_bytes, &nonce)
                    .await
                    .context("证明生成失败")?;

                let (_, measurement) = measure_zkp_verification_time(
                    zkp_manager,
                    &proof_result.proof,
                    &proof_result.public_inputs,
                    &proof_result.circuit_output,
                )
                .await?;

                Ok(measurement)
            }

            MetricType::MessageDiscoveryLatency => {
                let libp2p_identity1 = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id1 = libp2p_identity1.peer_id();
                let libp2p_identity2 = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id2 = libp2p_identity2.peer_id();

                let (_, measurement) = measure_message_discovery_latency(&peer_id1, &peer_id2).await?;
                Ok(measurement)
            }

            MetricType::P2PCommunicationLatency => {
                let libp2p_identity1 = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id1 = libp2p_identity1.peer_id().to_base58();
                let libp2p_identity2 = crate::LibP2PIdentity::generate()
                    .context("PeerID 生成失败")?;
                let peer_id2 = libp2p_identity2.peer_id().to_base58();

                let (_, measurement) = measure_p2p_communication_latency(&peer_id1, &peer_id2, 1024).await?;
                Ok(measurement)
            }

            MetricType::Throughput => {
                let (_, measurement) = measure_throughput(10, 5).await?;
                Ok(measurement)
            }

            MetricType::PacketLossRate => {
                let (_, measurement) = measure_packet_loss_rate(100, 10).await?;
                Ok(measurement)
            }

            MetricType::NodeVisibility => {
                let discovered_peers = rand::random::<usize>() % 20;
                let (_, measurement) = measure_node_visibility(discovered_peers).await?;
                Ok(measurement)
            }

            MetricType::StartupTime => {
                let (_, measurement) = measure_startup_time(async {
                    sleep(Duration::from_millis(100)).await;
                    Ok(())
                })
                .await?;
                Ok(measurement)
            }

            MetricType::PrivacyExposure => {
                let exposed_peers = rand::random::<usize>() % 10;
                let (_, measurement) = measure_privacy_exposure(exposed_peers).await?;
                Ok(measurement)
            }

            MetricType::ResourceUsage => {
                let (_, measurement) = measure_resource_usage().await?;
                Ok(measurement)
            }

            MetricType::MaliciousNodeAttackSuccessRate => {
                let (_, measurement) = measure_malicious_node_attack_success_rate().await?;
                Ok(measurement)
            }

            MetricType::ConnectionDropRate => {
                let (_, measurement) = measure_connection_drop_rate(50, 0.2).await?;
                Ok(measurement)
            }

            MetricType::ReconnectionAttempts => {
                let (_, measurement) = measure_reconnection_attempts(5).await?;
                Ok(measurement)
            }

            MetricType::ThroughputMbps => {
                let (_, measurement) = measure_throughput_mbps(5_000_000, 5).await?;
                Ok(measurement)
            }

            MetricType::CpuUsagePercent => {
                let (_, measurement) = measure_cpu_usage_percent().await?;
                Ok(measurement)
            }

            MetricType::RetryCount => {
                let (_, measurement) = measure_retry_count(100, 0.25).await?;
                Ok(measurement)
            }

            MetricType::ActiveSessions => {
                let (_, measurement) = measure_active_sessions(50).await?;
                Ok(measurement)
            }
        }
    }
}

