/**
 * DIAP Rust SDK - 实验基准测试模块
 * 用于测量和评估系统各项性能指标
 * 
 * 模块结构：
 * - types: 类型定义（指标类型、统计结果、配置等）
 * - collector: 指标收集器
 * - measurements: 各项指标的测量函数
 * - runner: 实验运行器
 * - reporter: 报告生成器
 */

pub mod types;
pub mod collector;
pub mod measurements;
pub mod runner;
pub mod reporter;

// 重新导出所有公共接口
pub use types::{
    ExperimentConfig, ExperimentResult, Measurement, MetricStatistics, MetricType,
    ResourceUsageMetrics,
};
pub use collector::MetricCollector;
pub use measurements::*;
pub use runner::ExperimentRunner;
pub use reporter::{ReportFormat, ReportGenerator};

