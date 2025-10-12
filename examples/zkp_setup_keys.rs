// DIAP Rust SDK - ZKP可信设置工具
// 生成proving key和verifying key

use diap_rs_sdk::ZKPSetup;
use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("\n🔧 DIAP ZKP可信设置工具\n");
    println!("========================================");
    println!("此工具将生成Groth16 proving key和verifying key");
    println!("警告：这是简化版可信设置，不适合生产环境");
    println!("生产环境应使用多方计算（MPC）进行可信设置");
    println!("========================================\n");
    
    // 1. 生成keys
    println!("📦 步骤1：生成proving key和verifying key");
    println!("  (这可能需要10-30秒，请耐心等待...)");
    println!();
    
    let (pk_bytes, vk_bytes) = ZKPSetup::generate_keys()?;
    
    println!("\n✅ Keys生成成功");
    println!("  Proving key大小: {} KB", pk_bytes.len() / 1024);
    println!("  Verifying key大小: {} bytes", vk_bytes.len());
    println!();
    
    // 2. 保存keys到文件
    println!("📦 步骤2：保存keys到文件");
    
    let pk_path = PathBuf::from("zkp_proving.key");
    let vk_path = PathBuf::from("zkp_verifying.key");
    
    ZKPSetup::save_keys_to_files(
        &pk_bytes,
        &vk_bytes,
        pk_path.to_str().unwrap(),
        vk_path.to_str().unwrap(),
    )?;
    
    println!();
    
    // 3. 验证keys（可选）
    println!("📦 步骤3：验证keys的有效性");
    println!("  (这将生成并验证一个测试证明...)");
    println!();
    
    let valid = ZKPSetup::verify_keys(&pk_bytes, &vk_bytes)?;
    
    if valid {
        println!("\n✅ Keys验证成功！");
    } else {
        println!("\n❌ Keys验证失败！");
        anyhow::bail!("生成的keys无法正常工作");
    }
    
    // 4. 使用说明
    println!();
    println!("========================================");
    println!("✅ 可信设置完成！");
    println!();
    println!("生成的文件：");
    println!("  • {}", pk_path.display());
    println!("  • {}", vk_path.display());
    println!();
    println!("使用方法：");
    println!("  let manager = IdentityManager::new_with_real_zkp(");
    println!("      ipfs_client,");
    println!("      ZKPProver::new(),");
    println!("      ZKPVerifier::new(),");
    println!("      \"{}\",", pk_path.display());
    println!("      \"{}\",", vk_path.display());
    println!("  )?;");
    println!();
    println!("⚠️  安全提示：");
    println!("  • 这些keys包含可信设置的随机数");
    println!("  • 在生产环境中，应使用MPC生成");
    println!("  • 不要重复使用测试环境的keys");
    println!("========================================\n");
    
    Ok(())
}

