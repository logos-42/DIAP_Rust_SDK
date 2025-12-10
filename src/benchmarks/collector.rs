/**
 * 实验基准测试 - 指标收集器模块
 */
use crate::benchmarks::types::{Measurement, MetricStatistics, MetricType};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 指标收集器
pub struct MetricCollector {
    /// 存储所有测量数据
    measurements: Arc<RwLock<HashMap<MetricType, VecDeque<Measurement>>>>,
    /// 最大存储的测量数量（防止内存溢出）
    max_measurements_per_metric: usize,
}

impl MetricCollector {
    /// 创建新的指标收集器
    pub fn new(max_measurements_per_metric: usize) -> Self {
        Self {
            measurements: Arc::new(RwLock::new(HashMap::new())),
            max_measurements_per_metric,
        }
    }

    /// 记录一次测量
    pub async fn record_measurement(
        &self,
        metric_type: MetricType,
        value: f64,
        metadata: HashMap<String, String>,
    ) {
        let measurement = Measurement {
            metric_type,
            value,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata,
        };

        let mut measurements = self.measurements.write().await;
        let queue = measurements.entry(metric_type).or_insert_with(VecDeque::new);
        queue.push_back(measurement);

        // 如果超过最大数量，移除最旧的
        while queue.len() > self.max_measurements_per_metric {
            queue.pop_front();
        }
    }

    /// 获取指定指标的统计结果
    pub async fn get_statistics(&self, metric_type: MetricType) -> MetricStatistics {
        let measurements = self.measurements.read().await;
        if let Some(queue) = measurements.get(&metric_type) {
            let samples: Vec<f64> = queue.iter().map(|m| m.value).collect();
            MetricStatistics::from_samples(&samples)
        } else {
            MetricStatistics::default()
        }
    }

    /// 获取所有原始测量数据
    pub async fn get_all_measurements(&self) -> Vec<Measurement> {
        let measurements = self.measurements.read().await;
        measurements
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    /// 清除所有测量数据
    pub async fn clear(&self) {
        let mut measurements = self.measurements.write().await;
        measurements.clear();
    }
}

