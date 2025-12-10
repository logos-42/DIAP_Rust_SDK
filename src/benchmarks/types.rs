/**
 * 实验基准测试 - 类型定义模块
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 实验指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// 注册延迟
    RegistrationLatency,
    /// ZKP 生成时间
    ZKPGenerationTime,
    /// ZKP 验证时间
    ZKPVerificationTime,
    /// 消息发现延迟
    MessageDiscoveryLatency,
    /// 点对点通信延迟
    P2PCommunicationLatency,
    /// 吞吐量
    Throughput,
    /// 丢包率 / 消息失败率
    PacketLossRate,
    /// 节点可见数 / 网络连通性
    NodeVisibility,
    /// 恶意节点/假身份攻击成功率
    MaliciousNodeAttackSuccessRate,
    /// 启动时间 / 同步时间
    StartupTime,
    /// 隐私暴露指标
    PrivacyExposure,
    /// 资源使用（CPU/内存/带宽）
    ResourceUsage,
    /// 连接掉线率
    ConnectionDropRate,
    /// 重连尝试次数
    ReconnectionAttempts,
    /// 网络吞吐（Mbps）
    ThroughputMbps,
    /// CPU 使用率（独立指标）
    CpuUsagePercent,
    /// 操作重试次数
    RetryCount,
    /// 活动会话数量
    ActiveSessions,
}

/// 统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStatistics {
    /// 测量次数
    pub count: usize,
    /// 平均值（毫秒或相应单位）
    pub mean: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 中位数 (P50)
    pub p50: f64,
    /// P95 百分位数
    pub p95: f64,
    /// P99 百分位数
    pub p99: f64,
    /// 标准差
    pub std_dev: f64,
}

impl MetricStatistics {
    /// 从样本数据计算统计信息
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = samples.len();
        let sum: f64 = samples.iter().sum();
        let mean = sum / count as f64;

        let variance: f64 = samples
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / count as f64;
        let std_dev = variance.sqrt();

        let min = sorted[0];
        let max = sorted[count - 1];

        let p50 = percentile(&sorted, 0.50);
        let p95 = percentile(&sorted, 0.95);
        let p99 = percentile(&sorted, 0.99);

        Self {
            count,
            mean,
            min,
            max,
            p50,
            p95,
            p99,
            std_dev,
        }
    }
}

impl Default for MetricStatistics {
    fn default() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            std_dev: 0.0,
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = (sorted.len() as f64 * p).ceil() as usize - 1;
    sorted[index.min(sorted.len() - 1)]
}

/// 单个测量结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    /// 指标类型
    pub metric_type: MetricType,
    /// 测量值（毫秒或相应单位）
    pub value: f64,
    /// 时间戳
    pub timestamp: String,
    /// 附加信息
    pub metadata: HashMap<String, String>,
}

/// 实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// 实验名称
    pub name: String,
    /// 要测量的指标类型列表
    pub metrics: Vec<MetricType>,
    /// 每个指标的重复测量次数
    pub iterations: usize,
    /// 节点数量（用于网络相关测试）
    pub node_count: usize,
    /// IPFS API URL
    pub ipfs_api_url: String,
    /// IPFS Gateway URL
    pub ipfs_gateway_url: String,
    /// 实验超时时间（秒）
    pub timeout_seconds: u64,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            name: "default_experiment".to_string(),
            metrics: vec![],
            iterations: 10,
            node_count: 2,
            ipfs_api_url: "http://127.0.0.1:5001".to_string(),
            ipfs_gateway_url: "http://127.0.0.1:8081".to_string(),
            timeout_seconds: 300,
        }
    }
}

/// 实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// 实验配置
    pub config: ExperimentConfig,
    /// 各指标的统计结果
    pub metrics: HashMap<String, MetricStatistics>,
    /// 所有原始测量数据
    pub raw_measurements: Vec<Measurement>,
    /// 实验开始时间
    pub start_time: String,
    /// 实验结束时间
    pub end_time: String,
    /// 总耗时（秒）
    pub duration_seconds: f64,
    /// 错误信息（如果有）
    pub errors: Vec<String>,
}

/// 资源使用指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageMetrics {
    /// CPU 使用率（百分比）
    pub cpu_percent: f64,
    /// 内存使用（MB）
    pub memory_mb: f64,
    /// 网络带宽使用（KB/s）
    pub bandwidth_kbps: f64,
}

