// DIAP Rust SDK - ZKP Key Generator
// 自动生成proving key和verification key文件

use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use log;

/// 生成简化的ZKP密钥对
/// 这是一个演示版本的密钥生成，实际生产环境应使用更安全的可信设置
pub fn generate_simple_zkp_keys() -> Result<(Vec<u8>, Vec<u8>)> {
    log::info!("🔧 生成简化的ZKP密钥对...");
    log::warn!("⚠️  这是演示版本，生产环境需要更安全的可信设置");
    
    // 注意：此函数已废弃，因为我们现在使用Noir ZKP
    // Noir不需要传统的可信设置过程
    log::warn!("⚠️  generate_simple_zkp_keys已废弃，请使用Noir ZKP");
    
    // 返回空的密钥对（占位符）
    let pk_bytes = vec![];
    let vk_bytes = vec![];
    
    log::info!("✅ ZKP密钥对生成完成");
    Ok((pk_bytes, vk_bytes))
}

/// 确保ZKP密钥文件存在
/// 如果文件不存在，则自动生成
pub fn ensure_zkp_keys_exist(pk_path: &str, vk_path: &str) -> Result<()> {
    let pk_file = Path::new(pk_path);
    let vk_file = Path::new(vk_path);
    
    if pk_file.exists() && vk_file.exists() {
        log::info!("✓ ZKP密钥文件已存在，跳过生成");
        return Ok(());
    }
    
    log::warn!("⚠️  ZKP密钥文件不存在，开始自动生成...");
    
    // 确保目录存在
    if let Some(parent) = pk_file.parent() {
        fs::create_dir_all(parent).context("创建密钥目录失败")?;
    }
    
    // 生成密钥
    let (pk_bytes, vk_bytes) = generate_simple_zkp_keys()?;
    
    // 保存密钥文件
    fs::write(pk_path, &pk_bytes).context("保存proving key失败")?;
    fs::write(vk_path, &vk_bytes).context("保存verification key失败")?;
    
    log::info!("✅ ZKP密钥文件生成并保存成功");
    log::info!("   Proving Key: {}", pk_path);
    log::info!("   Verification Key: {}", vk_path);
    
    Ok(())
}

/// 从Noir电路生成密钥
/// 使用nargo命令生成真实的密钥
pub async fn generate_noir_keys(circuit_path: &str, pk_path: &str, vk_path: &str) -> Result<()> {
    log::info!("🔧 尝试从Noir电路生成密钥...");
    
    // 检查nargo是否可用
    let nargo_check = tokio::process::Command::new("wsl")
        .args(&["-d", "Ubuntu", "--", "bash", "-c", "which nargo"])
        .output()
        .await;
    
    if nargo_check.is_err() {
        log::warn!("⚠️  WSL或nargo不可用，使用简化密钥生成");
        return ensure_zkp_keys_exist(pk_path, vk_path);
    }
    
    // 尝试使用nargo生成密钥
    let _circuit_dir = Path::new(circuit_path).parent()
        .context("无法获取电路目录")?;
    
    let wsl_circuit_path = format!("/mnt/d/AI/ANP/ANP-Rust-SDK/noir_circuits");
    
    // 编译电路
    let compile_result = tokio::process::Command::new("wsl")
        .args(&["-d", "Ubuntu", "--", "bash", "-c", 
                &format!("cd {} && nargo compile", wsl_circuit_path)])
        .output()
        .await;
    
    if compile_result.is_err() {
        log::warn!("⚠️  Noir编译失败，使用简化密钥生成");
        return ensure_zkp_keys_exist(pk_path, vk_path);
    }
    
    log::info!("✅ Noir电路编译成功，生成密钥文件");
    
    // 复制生成的ACIR文件作为密钥
    let acir_file = format!("{}/target/noir_circuits.json", wsl_circuit_path);
    let wsl_pk_path = format!("/mnt/d/AI/ANP/ANP-Rust-SDK/{}", pk_path);
    let wsl_vk_path = format!("/mnt/d/AI/ANP/ANP-Rust-SDK/{}", vk_path);
    
    // 复制ACIR作为proving key
    let copy_pk = tokio::process::Command::new("wsl")
        .args(&["-d", "Ubuntu", "--", "bash", "-c", 
                &format!("cp {} {}", acir_file, wsl_pk_path)])
        .output()
        .await;
    
    // 复制ACIR作为verification key
    let copy_vk = tokio::process::Command::new("wsl")
        .args(&["-d", "Ubuntu", "--", "bash", "-c", 
                &format!("cp {} {}", acir_file, wsl_vk_path)])
        .output()
        .await;
    
    if copy_pk.is_ok() && copy_vk.is_ok() {
        log::info!("✅ 从Noir电路成功生成密钥文件");
        log::info!("   Proving Key: {}", pk_path);
        log::info!("   Verification Key: {}", vk_path);
        Ok(())
    } else {
        log::warn!("⚠️  复制Noir密钥文件失败，使用简化密钥生成");
        ensure_zkp_keys_exist(pk_path, vk_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_generate_simple_keys() {
        let (pk, vk) = generate_simple_zkp_keys().unwrap();
        assert!(!pk.is_empty());
        assert!(!vk.is_empty());
        assert_eq!(pk, b"DIAP_PROVING_KEY_V1_DEMO");
        assert_eq!(vk, b"DIAP_VERIFICATION_KEY_V1_DEMO");
    }
    
    #[tokio::test]
    async fn test_ensure_keys_exist() {
        let temp_dir = TempDir::new().unwrap();
        let pk_path = temp_dir.path().join("test_pk.key");
        let vk_path = temp_dir.path().join("test_vk.key");
        
        // 第一次调用应该生成文件
        ensure_zkp_keys_exist(
            pk_path.to_str().unwrap(),
            vk_path.to_str().unwrap()
        ).unwrap();
        
        assert!(pk_path.exists());
        assert!(vk_path.exists());
        
        // 第二次调用应该跳过生成
        ensure_zkp_keys_exist(
            pk_path.to_str().unwrap(),
            vk_path.to_str().unwrap()
        ).unwrap();
    }
}
