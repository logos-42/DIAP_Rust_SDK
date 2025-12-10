/**
 * 实验基准测试 - 报告生成器模块
 */
use crate::benchmarks::types::ExperimentResult;
use anyhow::{Context, Result};
use std::cmp::min;

/// 报告格式
#[derive(Debug, Clone, Copy)]
pub enum ReportFormat {
    /// 文本格式
    Text,
    /// JSON 格式
    Json,
}

/// 实验报告生成器
pub struct ReportGenerator;

impl ReportGenerator {
    /// 生成文本格式的报告
    pub fn generate_text_report(result: &ExperimentResult) -> String {
        let mut report = String::new();

        report.push_str("═══════════════════════════════════════════════════════════\n");
        report.push_str(&format!("实验报告: {}\n", result.config.name));
        report.push_str("═══════════════════════════════════════════════════════════\n\n");

        report.push_str(&format!("开始时间: {}\n", result.start_time));
        report.push_str(&format!("结束时间: {}\n", result.end_time));
        report.push_str(&format!("总耗时: {:.2} 秒\n", result.duration_seconds));
        report.push_str(&format!("错误数量: {}\n\n", result.errors.len()));

        if !result.errors.is_empty() {
            report.push_str("错误信息:\n");
            for error in &result.errors {
                report.push_str(&format!("  - {}\n", error));
            }
            report.push_str("\n");
        }

        report.push_str("═══════════════════════════════════════════════════════════\n");
        report.push_str("指标统计结果\n");
        report.push_str("═══════════════════════════════════════════════════════════\n\n");

        for (metric_name, stats) in &result.metrics {
            report.push_str(&format!("指标: {}\n", metric_name));
            report.push_str(&format!("  测量次数: {}\n", stats.count));
            report.push_str(&format!("  平均值: {:.2}\n", stats.mean));
            report.push_str(&format!("  最小值: {:.2}\n", stats.min));
            report.push_str(&format!("  最大值: {:.2}\n", stats.max));
            report.push_str(&format!("  中位数 (P50): {:.2}\n", stats.p50));
            report.push_str(&format!("  P95: {:.2}\n", stats.p95));
            report.push_str(&format!("  P99: {:.2}\n", stats.p99));
            report.push_str(&format!("  标准差: {:.2}\n", stats.std_dev));
            report.push_str("\n");
        }

        report.push_str("═══════════════════════════════════════════════════════════\n");
        report.push_str("原始测量样本（最多展示 10 条）\n");
        report.push_str("═══════════════════════════════════════════════════════════\n\n");

        if result.raw_measurements.is_empty() {
            report.push_str("暂无原始测量样本。\n");
        } else {
            let sample_count = min(10, result.raw_measurements.len());
            for measurement in result.raw_measurements.iter().take(sample_count) {
                report.push_str(&format!(
                    "  [{}] {:?}: {:.2} (metadata: {})\n",
                    measurement.timestamp,
                    measurement.metric_type,
                    measurement.value,
                    measurement.metadata.len()
                ));
            }
            report.push_str(&format!(
                "\n共记录 {} 条原始样本。\n",
                result.raw_measurements.len()
            ));
        }

        report.push_str("\n");

        report
    }

    /// 生成 JSON 格式的报告
    pub fn generate_json_report(result: &ExperimentResult) -> Result<String> {
        serde_json::to_string_pretty(result).context("JSON 序列化失败")
    }

    /// 保存报告到文件
    pub async fn save_report(
        result: &ExperimentResult,
        file_path: &str,
        format: ReportFormat,
    ) -> Result<()> {
        let content = match format {
            ReportFormat::Text => Self::generate_text_report(result),
            ReportFormat::Json => Self::generate_json_report(result)?,
        };

        tokio::fs::write(file_path, content)
            .await
            .context(format!("保存报告失败: {}", file_path))?;

        log::info!("📄 报告已保存到: {}", file_path);
        Ok(())
    }
}

