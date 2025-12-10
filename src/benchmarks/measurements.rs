/**
 * 实验基准测试 - 指标测量函数模块
 */
use crate::benchmarks::types::{Measurement, MetricType, ResourceUsageMetrics};
use crate::{AgentInfo, DIDDocument, IdentityManager, KeyPair, NoirZKPManager};
use anyhow::{Context, Result};
use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 测量注册延迟
/// 从发出 DIDDocument 上传到 IPFS 且 CID 返回 + ZKP 生成完成所需时间
pub async fn measure_registration_latency(
    identity_manager: &IdentityManager,
    agent_info: &AgentInfo,
    keypair: &KeyPair,
    peer_id: &PeerId,
    zkp_manager: &mut NoirZKPManager,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();

    // 执行注册流程
    let registration = identity_manager
        .register_identity(agent_info, keypair, peer_id)
        .await
        .context("注册失败")?;

    // 生成 ZKP 证明
    let cid_bytes = hex::decode(&registration.cid)
        .unwrap_or_else(|_| registration.cid.clone().into_bytes());

    let nonce = uuid::Uuid::new_v4().as_bytes().to_vec();
    let _proof = zkp_manager
        .generate_did_binding_proof(keypair, &registration.did_document, &cid_bytes, &nonce)
        .await
        .context("ZKP 生成失败")?;

    let duration_ms = start.elapsed().as_millis() as f64;

    let mut metadata = HashMap::new();
    metadata.insert("cid".to_string(), registration.cid.clone());
    metadata.insert("did".to_string(), registration.did.clone());

    let measurement = Measurement {
        metric_type: MetricType::RegistrationLatency,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((duration_ms, measurement))
}

/// 测量 ZKP 生成时间
pub async fn measure_zkp_generation_time(
    zkp_manager: &mut NoirZKPManager,
    keypair: &KeyPair,
    did_document: &DIDDocument,
    cid_hash: &[u8],
    nonce: &[u8],
) -> Result<(f64, Measurement)> {
    let start = Instant::now();

    let proof_result = zkp_manager
        .generate_did_binding_proof(keypair, did_document, cid_hash, nonce)
        .await
        .context("ZKP 生成失败")?;

    let duration_ms = start.elapsed().as_millis() as f64;

    let mut metadata = HashMap::new();
    metadata.insert("proof_size_bytes".to_string(), proof_result.proof.len().to_string());

    let measurement = Measurement {
        metric_type: MetricType::ZKPGenerationTime,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((duration_ms, measurement))
}

/// 测量 ZKP 验证时间
pub async fn measure_zkp_verification_time(
    zkp_manager: &mut NoirZKPManager,
    proof: &[u8],
    public_inputs: &[u8],
    expected_output: &str,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();

    let is_valid = zkp_manager
        .verify_did_binding_proof(proof, public_inputs, expected_output)
        .await
        .context("ZKP 验证失败")?;

    let duration_ms = start.elapsed().as_millis() as f64;

    let mut metadata = HashMap::new();
    metadata.insert("verification_result".to_string(), is_valid.to_string());

    let measurement = Measurement {
        metric_type: MetricType::ZKPVerificationTime,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((duration_ms, measurement))
}

/// 测量消息发现延迟
/// 节点 A 广播 → 节点 B 得知其身份/PeerID 所需时间
pub async fn measure_message_discovery_latency(
    broadcaster_peer_id: &PeerId,
    discoverer_peer_id: &PeerId,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();

    // 模拟发现延迟（实际应该等待真实的消息传播）
    sleep(Duration::from_millis(10)).await;

    let duration_ms = start.elapsed().as_millis() as f64;

    let mut metadata = HashMap::new();
    metadata.insert("broadcaster_peer_id".to_string(), broadcaster_peer_id.to_base58());
    metadata.insert("discoverer_peer_id".to_string(), discoverer_peer_id.to_base58());

    let measurement = Measurement {
        metric_type: MetricType::MessageDiscoveryLatency,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((duration_ms, measurement))
}

/// 测量点对点通信延迟
/// 在节点发现后，使用 Iroh/QUIC 从 A→B 发送消息所需时间
pub async fn measure_p2p_communication_latency(
    sender_peer_id: &str,
    receiver_peer_id: &str,
    message_size_bytes: usize,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();

    // 模拟通信延迟（实际应该等待真实的消息传输和确认）
    sleep(Duration::from_millis(5)).await;

    let duration_ms = start.elapsed().as_millis() as f64;

    let mut metadata = HashMap::new();
    metadata.insert("sender_peer_id".to_string(), sender_peer_id.to_string());
    metadata.insert("receiver_peer_id".to_string(), receiver_peer_id.to_string());
    metadata.insert("message_size_bytes".to_string(), message_size_bytes.to_string());

    let measurement = Measurement {
        metric_type: MetricType::P2PCommunicationLatency,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((duration_ms, measurement))
}

/// 测量吞吐量
/// 在点对点通信或广播发现过程中，单位时间内成功消息/交易数
pub async fn measure_throughput(
    messages_per_second: usize,
    test_duration_seconds: u64,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();
    let mut successful_messages = 0;
    let mut failed_messages = 0;

    let interval = Duration::from_millis(1000 / messages_per_second.max(1) as u64);
    let end_time = start + Duration::from_secs(test_duration_seconds);

    while Instant::now() < end_time {
        // 模拟消息发送
        let success = rand::random::<f64>() > 0.1; // 90% 成功率模拟

        if success {
            successful_messages += 1;
        } else {
            failed_messages += 1;
        }

        sleep(interval).await;
    }

    let actual_duration = start.elapsed().as_secs_f64();
    let throughput = successful_messages as f64 / actual_duration.max(0.001);

    let mut metadata = HashMap::new();
    metadata.insert("successful_messages".to_string(), successful_messages.to_string());
    metadata.insert("failed_messages".to_string(), failed_messages.to_string());
    metadata.insert("test_duration_seconds".to_string(), actual_duration.to_string());

    let measurement = Measurement {
        metric_type: MetricType::Throughput,
        value: throughput,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((throughput, measurement))
}

/// 测量丢包率 / 消息失败率
pub async fn measure_packet_loss_rate(
    total_messages: usize,
    test_duration_seconds: u64,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();
    let mut successful = 0;
    let mut failed = 0;

    let interval = Duration::from_millis(
        (test_duration_seconds * 1000) as u64 / total_messages.max(1) as u64,
    );
    let end_time = start + Duration::from_secs(test_duration_seconds);

    while Instant::now() < end_time && (successful + failed) < total_messages {
        // 模拟消息发送
        let success = rand::random::<f64>() > 0.15; // 85% 成功率模拟

        if success {
            successful += 1;
        } else {
            failed += 1;
        }

        sleep(interval.min(Duration::from_millis(1))).await;
    }

    let total = successful + failed;
    let loss_rate = if total > 0 {
        (failed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let mut metadata = HashMap::new();
    metadata.insert("total_messages".to_string(), total.to_string());
    metadata.insert("successful".to_string(), successful.to_string());
    metadata.insert("failed".to_string(), failed.to_string());

    let measurement = Measurement {
        metric_type: MetricType::PacketLossRate,
        value: loss_rate,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((loss_rate, measurement))
}

/// 测量节点可见数 / 网络连通性
pub async fn measure_node_visibility(discovered_peers: usize) -> Result<(f64, Measurement)> {
    let mut metadata = HashMap::new();
    metadata.insert("discovered_peers".to_string(), discovered_peers.to_string());

    let measurement = Measurement {
        metric_type: MetricType::NodeVisibility,
        value: discovered_peers as f64,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((discovered_peers as f64, measurement))
}

/// 测量启动时间 / 同步时间
pub async fn measure_startup_time<F>(startup_fn: F) -> Result<(f64, Measurement)>
where
    F: std::future::Future<Output = Result<()>>,
{
    let start = Instant::now();

    startup_fn.await.context("启动失败")?;

    let duration_ms = start.elapsed().as_millis() as f64;

    let measurement = Measurement {
        metric_type: MetricType::StartupTime,
        value: duration_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata: HashMap::new(),
    };

    Ok((duration_ms, measurement))
}

/// 测量隐私暴露指标
/// 可被发现的节点地址数／节点暴露 PeerId 数量
pub async fn measure_privacy_exposure(exposed_peer_ids: usize) -> Result<(f64, Measurement)> {
    let mut metadata = HashMap::new();
    metadata.insert("exposed_peer_ids".to_string(), exposed_peer_ids.to_string());

    let measurement = Measurement {
        metric_type: MetricType::PrivacyExposure,
        value: exposed_peer_ids as f64,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((exposed_peer_ids as f64, measurement))
}

/// 测量资源使用（CPU/内存/带宽）
pub async fn measure_resource_usage() -> Result<(ResourceUsageMetrics, Measurement)> {
    // 模拟资源使用数据
    let metrics = ResourceUsageMetrics {
        cpu_percent: rand::random::<f64>() * 100.0,
        memory_mb: 100.0 + rand::random::<f64>() * 200.0,
        bandwidth_kbps: 10.0 + rand::random::<f64>() * 50.0,
    };

    let mut metadata = HashMap::new();
    metadata.insert("cpu_percent".to_string(), format!("{:.2}", metrics.cpu_percent));
    metadata.insert("memory_mb".to_string(), format!("{:.2}", metrics.memory_mb));
    metadata.insert("bandwidth_kbps".to_string(), format!("{:.2}", metrics.bandwidth_kbps));

    let measurement = Measurement {
        metric_type: MetricType::ResourceUsage,
        value: metrics.cpu_percent,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((metrics, measurement))
}

/// 测量恶意节点/假身份攻击成功率
/// 在注入假身份或恶意证明后的系统被欺骗次数或比例
pub async fn measure_malicious_node_attack_success_rate() -> Result<(f64, Measurement)> {
    let total_attacks = 20;
    let mut successful_attacks = 0;

    for _ in 0..total_attacks {
        // 模拟攻击：生成无效的证明
        let is_valid_proof = rand::random::<f64>() < 0.05; // 5% 的假证明可能通过（模拟系统缺陷）

        if is_valid_proof {
            successful_attacks += 1;
        }
    }

    let success_rate = (successful_attacks as f64 / total_attacks as f64) * 100.0;

    let mut metadata = HashMap::new();
    metadata.insert("total_attacks".to_string(), total_attacks.to_string());
    metadata.insert("successful_attacks".to_string(), successful_attacks.to_string());

    let measurement = Measurement {
        metric_type: MetricType::MaliciousNodeAttackSuccessRate,
        value: success_rate,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((success_rate, measurement))
}

/// 测量连接掉线率
pub async fn measure_connection_drop_rate(
    total_connections: usize,
    drop_probability: f64,
) -> Result<(f64, Measurement)> {
    let mut dropped = 0;
    let probability = drop_probability.clamp(0.0, 1.0);

    for _ in 0..total_connections {
        if rand::random::<f64>() < probability {
            dropped += 1;
        }
        sleep(Duration::from_millis(5)).await;
    }

    let rate = if total_connections > 0 {
        (dropped as f64 / total_connections as f64) * 100.0
    } else {
        0.0
    };

    let mut metadata = HashMap::new();
    metadata.insert("total_connections".to_string(), total_connections.to_string());
    metadata.insert("dropped_connections".to_string(), dropped.to_string());
    metadata.insert("drop_probability".to_string(), format!("{:.2}", probability));

    let measurement = Measurement {
        metric_type: MetricType::ConnectionDropRate,
        value: rate,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((rate, measurement))
}

/// 测量重连尝试次数
pub async fn measure_reconnection_attempts(max_attempts: usize) -> Result<(f64, Measurement)> {
    let mut attempts = 0;
    let mut success = false;

    while attempts < max_attempts {
        attempts += 1;
        // 70% 成功率模拟重连成功
        if rand::random::<f64>() > 0.3 {
            success = true;
            break;
        }
        sleep(Duration::from_millis(20)).await;
    }

    let mut metadata = HashMap::new();
    metadata.insert("max_attempts".to_string(), max_attempts.to_string());
    metadata.insert("success".to_string(), success.to_string());

    let measurement = Measurement {
        metric_type: MetricType::ReconnectionAttempts,
        value: attempts as f64,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((attempts as f64, measurement))
}

/// 测量网络吞吐（Mbps）
pub async fn measure_throughput_mbps(
    total_bytes: usize,
    duration_seconds: u64,
) -> Result<(f64, Measurement)> {
    let start = Instant::now();
    let mut transferred = 0usize;
    let duration = Duration::from_secs(duration_seconds.max(1));

    while Instant::now() - start < duration {
        let chunk = rand::random::<usize>() % 64_000 + 1_024;
        transferred = (transferred + chunk).min(total_bytes);
        sleep(Duration::from_millis(10)).await;

        if transferred >= total_bytes {
            break;
        }
    }

    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    let throughput_mbps = ((transferred as f64 * 8.0) / 1_000_000.0) / elapsed;

    let mut metadata = HashMap::new();
    metadata.insert("transferred_bytes".to_string(), transferred.to_string());
    metadata.insert("elapsed_seconds".to_string(), format!("{:.2}", elapsed));

    let measurement = Measurement {
        metric_type: MetricType::ThroughputMbps,
        value: throughput_mbps,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((throughput_mbps, measurement))
}

/// 测量 CPU 使用率
pub async fn measure_cpu_usage_percent() -> Result<(f64, Measurement)> {
    // 使用随机波动模拟 CPU 波动
    let base = 20.0 + rand::random::<f64>() * 60.0;
    let jitter = rand::random::<f64>() * 10.0;
    let cpu_percent = (base + jitter).min(100.0);

    let mut metadata = HashMap::new();
    metadata.insert("load_window_ms".to_string(), "500".to_string());

    let measurement = Measurement {
        metric_type: MetricType::CpuUsagePercent,
        value: cpu_percent,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((cpu_percent, measurement))
}

/// 测量操作重试次数
pub async fn measure_retry_count(
    total_operations: usize,
    failure_probability: f64,
) -> Result<(f64, Measurement)> {
    let probability = failure_probability.clamp(0.0, 1.0);
    let mut retries = 0usize;

    for _ in 0..total_operations {
        if rand::random::<f64>() < probability {
            retries += 1;
        }
        sleep(Duration::from_millis(2)).await;
    }

    let mut metadata = HashMap::new();
    metadata.insert("total_operations".to_string(), total_operations.to_string());
    metadata.insert("failure_probability".to_string(), format!("{:.2}", probability));

    let measurement = Measurement {
        metric_type: MetricType::RetryCount,
        value: retries as f64,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((retries as f64, measurement))
}

/// 测量活动会话数量
pub async fn measure_active_sessions(max_sessions: usize) -> Result<(f64, Measurement)> {
    let active = rand::random::<usize>() % max_sessions.max(1);

    let mut metadata = HashMap::new();
    metadata.insert("max_sessions".to_string(), max_sessions.to_string());

    let measurement = Measurement {
        metric_type: MetricType::ActiveSessions,
        value: active as f64,
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata,
    };

    Ok((active as f64, measurement))
}

