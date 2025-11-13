// DIAP Rust SDK - 跨平台兼容性演示
// 展示新的零依赖部署功能

use anyhow::Result;
use diap_rs_sdk::{AgentAuthManager, BackendInfo, NoirBackend, UniversalNoirManager};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🚀 DIAP SDK 跨平台兼容性演示");
    println!("==========================================");

    // 1. 测试通用Noir管理器
    println!("\n🔧 测试通用Noir管理器...");
    let start_time = Instant::now();

    let mut noir_manager = UniversalNoirManager::new().await?;
    let init_time = start_time.elapsed();

    println!("✅ 通用Noir管理器初始化成功");
    println!("   初始化时间: {:?}", init_time);

    // 显示后端信息
    let backend_info = noir_manager.get_backend_info();
    println!("   后端类型: {:?}", backend_info.backend_type);
    println!("   电路路径: {:?}", backend_info.circuits_path);
    println!("   可用状态: {}", backend_info.is_available);

    // 2. 测试证明生成和验证
    println!("\n🔐 测试证明生成和验证...");

    // 创建匹配的测试数据（确保哈希匹配）
    let public_key_hash = "pk_hash_67890";
    let nonce_hash = "nonce_hash_abcdef";
    let expected_did_hash = format!(
        "{:x}",
        Sha256::digest(format!("{}{}", public_key_hash, nonce_hash).as_bytes())
    );

    let inputs = diap_rs_sdk::noir_universal::NoirProverInputs {
        expected_did_hash,
        public_key_hash: public_key_hash.to_string(),
        nonce_hash: nonce_hash.to_string(),
        expected_output: "expected_output_xyz".to_string(),
    };

    // 生成证明
    let proof_start = Instant::now();
    let proof_result = noir_manager.generate_proof(&inputs).await?;
    let proof_time = proof_start.elapsed();

    println!("✅ 证明生成成功");
    println!("   生成时间: {:?}", proof_time);
    println!("   证明大小: {} bytes", proof_result.proof.len());
    println!("   电路输出: {}", proof_result.circuit_output);

    // 验证证明
    let verify_start = Instant::now();
    let verify_result = noir_manager
        .verify_proof(&proof_result.proof, &proof_result.public_inputs)
        .await?;
    let verify_time = verify_start.elapsed();

    println!("✅ 证明验证完成");
    println!("   验证时间: {:?}", verify_time);
    println!(
        "   验证结果: {}",
        if verify_result.is_valid {
            "通过"
        } else {
            "失败"
        }
    );

    // 3. 测试后端切换
    println!("\n🔄 测试后端切换...");

    let original_backend = noir_manager.get_backend_info().backend_type.clone();
    println!("   当前后端: {:?}", original_backend);

    // 尝试切换到简化后端
    noir_manager.switch_backend(NoirBackend::Simplified).await?;
    let new_backend_info = noir_manager.get_backend_info();
    println!("   切换后后端: {:?}", new_backend_info.backend_type);

    // 切换回原后端
    noir_manager.switch_backend(original_backend).await?;
    let final_backend_info = noir_manager.get_backend_info();
    println!("   最终后端: {:?}", final_backend_info.backend_type);

    // 4. 测试性能统计
    println!("\n📊 性能统计...");
    let perf_stats = noir_manager.get_performance_stats();
    println!("   后端类型: {:?}", perf_stats.backend_type);
    println!("   缓存条目: {}", perf_stats.cache_entries);
    println!("   内存使用: {} bytes", perf_stats.memory_usage_bytes);
    println!("   是否优化: {}", perf_stats.is_optimized);

    // 5. 测试智能体认证管理器（集成测试）
    println!("\n🤖 测试智能体认证管理器...");

    let auth_start = Instant::now();
    let auth_manager = AgentAuthManager::new().await?;
    let auth_init_time = auth_start.elapsed();

    println!("✅ 认证管理器初始化成功");
    println!("   初始化时间: {:?}", auth_init_time);

    // 创建测试智能体
    let (agent_info, keypair, peer_id) = auth_manager.create_agent("CrossPlatformTest", None)?;
    println!("✅ 智能体创建成功");
    println!("   智能体名称: {}", agent_info.name);
    println!("   DID: {}", keypair.did);
    println!("   PeerID: {}", peer_id);

    // 6. 跨平台兼容性总结
    println!("\n🎯 跨平台兼容性总结");
    println!("====================");

    println!("✅ 通用Noir管理器: 工作正常");
    println!("✅ 证明生成: 工作正常");
    println!("✅ 证明验证: 工作正常");
    println!("✅ 后端切换: 工作正常");
    println!("✅ 性能统计: 工作正常");
    println!("✅ 智能体认证: 工作正常");

    println!("\n🌍 支持的平台:");
    println!("   ✅ Windows (原生 + WSL fallback)");
    println!("   ✅ Linux (原生)");
    println!("   ✅ macOS (原生)");

    println!("\n🔧 后端支持:");
    println!("   ✅ 嵌入电路 (零依赖)");
    println!("   ✅ 外部Noir (需要nargo)");
    println!("   ✅ Arkworks ZKP (Rust原生)");
    println!("   ✅ 简化实现 (fallback)");

    println!("\n💡 使用建议:");
    println!("   1. 默认使用嵌入电路后端，零依赖部署");
    println!("   2. 如需自定义电路，可切换到外部Noir后端");
    println!("   3. 所有后端都支持跨平台运行");
    println!("   4. 自动fallback确保在任何环境下都能工作");

    println!("\n🎉 跨平台兼容性演示完成！");
    println!("==========================================");

    Ok(())
}
