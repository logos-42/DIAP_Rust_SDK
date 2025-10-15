// DIAP Rust SDK - 统一ZKP演示
// 展示如何使用统一接口解决Noir和Arkworks功能错位问题

use diap_rs_sdk::{
    UnifiedZKPManager,
    UnifiedZKPInputs,
    ZKPScheme,
    ZKPPerformanceTester,
    KeyPair,
};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 设置日志
    env_logger::init();
    
    println!("🚀 统一ZKP接口演示");
    println!("==========================================");
    
    // 1. 准备测试数据
    println!("\n📝 准备测试数据...");
    let keypair = KeyPair::generate()?;
    
    let inputs = UnifiedZKPInputs {
        secret_key: keypair.private_key.clone(),
        did_document: format!(r#"{{"id":"{}","verificationMethod":[]}}"#, keypair.did),
        nonce: b"test_nonce_123".to_vec(),
        cid_hash: b"test_cid_hash_456".to_vec(),
        expected_public_key: keypair.public_key.clone(),
    };
    
    println!("✅ 测试数据准备完成");
    println!("   DID: {}", keypair.did);
    println!("   私钥长度: {} bytes", inputs.secret_key.len());
    println!("   Nonce: {}", String::from_utf8_lossy(&inputs.nonce));
    
    // 2. 测试Noir方案
    println!("\n🔹 测试Noir ZKP方案...");
    let mut noir_manager = UnifiedZKPManager::new(
        ZKPScheme::Noir,
        "./noir_circuits".to_string(),
        "./arkworks_keys".to_string(),
    );
    
    match noir_manager.generate_proof(&inputs).await {
        Ok(output) => {
            println!("✅ Noir证明生成成功");
            println!("   证明大小: {} bytes", output.proof.len());
            println!("   生成时间: {}ms", output.generation_time_ms);
            println!("   电路输出: {}", output.circuit_output);
            
            // 验证证明
            match noir_manager.verify_proof(
                &output.proof,
                &output.public_inputs,
                &output.circuit_output,
            ).await {
                Ok(result) => {
                    println!("✅ Noir证明验证: {}", if result.is_valid { "成功" } else { "失败" });
                    if let Some(error) = result.error_message {
                        println!("   错误: {}", error);
                    }
                }
                Err(e) => println!("❌ Noir证明验证失败: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Noir证明生成失败: {}", e);
            println!("   这可能是因为WSL环境或Noir配置问题");
        }
    }
    
    // 3. 测试Arkworks方案
    println!("\n🔸 测试Arkworks ZKP方案...");
    let mut arkworks_manager = UnifiedZKPManager::new(
        ZKPScheme::Arkworks,
        "./noir_circuits".to_string(),
        "./arkworks_keys".to_string(),
    );
    
    match arkworks_manager.generate_proof(&inputs).await {
        Ok(output) => {
            println!("✅ Arkworks证明生成成功");
            println!("   证明大小: {} bytes", output.proof.len());
            println!("   生成时间: {}ms", output.generation_time_ms);
            
            // 验证证明
            match arkworks_manager.verify_proof(
                &output.proof,
                &output.public_inputs,
                &output.circuit_output,
            ).await {
                Ok(result) => {
                    println!("✅ Arkworks证明验证: {}", if result.is_valid { "成功" } else { "失败" });
                    if let Some(error) = result.error_message {
                        println!("   错误: {}", error);
                    }
                }
                Err(e) => println!("❌ Arkworks证明验证失败: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Arkworks证明生成失败: {}", e);
        }
    }
    
    // 4. 性能对比测试
    println!("\n📊 运行性能对比测试...");
    let mut tester = ZKPPerformanceTester::new(
        "./noir_circuits".to_string(),
        "./arkworks_keys".to_string(),
    );
    
    match tester.run_performance_test(&inputs).await {
        Ok(comparison) => {
            println!("✅ 性能对比测试完成");
            comparison.print_comparison_report();
        }
        Err(e) => {
            println!("❌ 性能对比测试失败: {}", e);
        }
    }
    
    // 5. 方案切换演示
    println!("\n🔄 方案切换演示...");
    let mut manager = UnifiedZKPManager::new(
        ZKPScheme::Noir,
        "./noir_circuits".to_string(),
        "./arkworks_keys".to_string(),
    );
    
    println!("当前方案: {:?}", manager.get_current_scheme());
    
    manager.switch_scheme(ZKPScheme::Arkworks);
    println!("切换到: {:?}", manager.get_current_scheme());
    
    manager.switch_scheme(ZKPScheme::Noir);
    println!("切换回: {:?}", manager.get_current_scheme());
    
    // 6. 错误处理演示
    println!("\n⚠️  错误处理演示...");
    
    // 测试无效输入
    let invalid_inputs = UnifiedZKPInputs {
        secret_key: vec![], // 空私钥
        did_document: "".to_string(),
        nonce: vec![],
        cid_hash: vec![],
        expected_public_key: vec![],
    };
    
    match noir_manager.generate_proof(&invalid_inputs).await {
        Ok(_) => println!("❌ 应该失败但成功了"),
        Err(e) => println!("✅ 正确处理了无效输入: {}", e),
    }
    
    println!("\n🎉 统一ZKP接口演示完成！");
    println!("==========================================");
    
    println!("\n💡 解决的问题:");
    println!("   ✅ 消除了Noir和Arkworks之间的功能错位");
    println!("   ✅ 提供了统一的API接口");
    println!("   ✅ 支持运行时方案切换");
    println!("   ✅ 提供了性能对比工具");
    println!("   ✅ 统一了数据格式和错误处理");
    
    Ok(())
}
