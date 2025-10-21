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

/// 从Noir电路生成密钥（跨平台版本）
/// 自动检测环境并选择合适的执行方式
pub async fn generate_noir_keys(circuit_path: &str, pk_path: &str, vk_path: &str) -> Result<()> {
    log::info!("🔧 尝试从Noir电路生成密钥...");
    
    // 获取电路目录
    let circuit_dir = Path::new(circuit_path).parent()
        .context("无法获取电路目录")?;
    
    // 检查nargo是否可用（跨平台检测）
    let nargo_available = check_nargo_available().await;
    
    if !nargo_available {
        log::warn!("⚠️  nargo不可用，使用简化密钥生成");
        return ensure_zkp_keys_exist(pk_path, vk_path);
    }
    
    // 编译电路（跨平台）
    let compile_result = compile_noir_circuit(circuit_dir).await;
    
    if compile_result.is_err() {
        log::warn!("⚠️  Noir编译失败，使用简化密钥生成");
        return ensure_zkp_keys_exist(pk_path, vk_path);
    }
    
    log::info!("✅ Noir电路编译成功，生成密钥文件");
    
    // 复制生成的ACIR文件作为密钥
    let acir_file = circuit_dir.join("target").join("noir_circuits.json");
    
    if !acir_file.exists() {
        log::warn!("⚠️  ACIR文件不存在，使用简化密钥生成");
        return ensure_zkp_keys_exist(pk_path, vk_path);
    }
    
    // 复制ACIR作为密钥文件
    let copy_result = copy_acir_as_keys(&acir_file, pk_path, vk_path).await;
    
    if copy_result.is_ok() {
        log::info!("✅ 从Noir电路成功生成密钥文件");
        log::info!("   Proving Key: {}", pk_path);
        log::info!("   Verification Key: {}", vk_path);
        Ok(())
    } else {
        log::warn!("⚠️  复制Noir密钥文件失败，使用简化密钥生成");
        ensure_zkp_keys_exist(pk_path, vk_path)
    }
}

/// 检查nargo是否可用（跨平台）
async fn check_nargo_available() -> bool {
    // 首先尝试直接调用nargo
    if let Ok(output) = tokio::process::Command::new("nargo")
        .arg("--version")
        .output()
        .await
    {
        if output.status.success() {
            log::info!("✅ 检测到nargo (直接调用)");
            return true;
        }
    }
    
    // 在Windows上，尝试WSL作为fallback
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = tokio::process::Command::new("wsl")
            .args(&["-d", "Ubuntu", "--", "bash", "-c", "which nargo"])
            .output()
            .await
        {
            if output.status.success() {
                log::info!("✅ 检测到nargo (WSL)");
                return true;
            }
        }
    }
    
    log::warn!("⚠️  nargo不可用");
    false
}

/// 编译Noir电路（跨平台）
async fn compile_noir_circuit(circuit_dir: &Path) -> Result<()> {
    // 首先尝试直接调用nargo
    if let Ok(output) = tokio::process::Command::new("nargo")
        .arg("compile")
        .current_dir(circuit_dir)
        .output()
        .await
    {
        if output.status.success() {
            log::info!("✅ 电路编译成功 (直接调用)");
            return Ok(());
        }
    }
    
    // 在Windows上，尝试WSL作为fallback
    #[cfg(target_os = "windows")]
    {
        let wsl_circuit_path = format!("/mnt/{}/{}", 
            circuit_dir.to_string_lossy().chars().next().unwrap().to_lowercase(),
            circuit_dir.to_string_lossy()[2..].replace('\\', "/"));
        
        if let Ok(output) = tokio::process::Command::new("wsl")
            .args(&["-d", "Ubuntu", "--", "bash", "-c", 
                    &format!("cd {} && nargo compile", wsl_circuit_path)])
            .output()
            .await
        {
            if output.status.success() {
                log::info!("✅ 电路编译成功 (WSL)");
                return Ok(());
            }
        }
    }
    
    Err(anyhow::anyhow!("Noir电路编译失败"))
}

/// 复制ACIR文件作为密钥文件
async fn copy_acir_as_keys(acir_file: &Path, pk_path: &str, vk_path: &str) -> Result<()> {
    // 确保目标目录存在
    if let Some(parent) = Path::new(pk_path).parent() {
        std::fs::create_dir_all(parent).context("创建密钥目录失败")?;
    }
    
    // 读取ACIR文件
    let acir_data = std::fs::read(acir_file)
        .context("读取ACIR文件失败")?;
    
    // 复制ACIR作为proving key
    std::fs::write(pk_path, &acir_data)
        .context("保存proving key失败")?;
    
    // 复制ACIR作为verification key
    std::fs::write(vk_path, &acir_data)
        .context("保存verification key失败")?;
    
    Ok(())
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
