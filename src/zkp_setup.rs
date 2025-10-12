// DIAP Rust SDK - ZKP可信设置模块
// 生成Groth16 proving和verifying keys

use anyhow::{Context, Result};
use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_std::rand::thread_rng;
use std::fs::File;
use std::io::BufWriter;
use crate::zkp_circuit::DIDBindingCircuit;

/// ZKP可信设置管理器
pub struct ZKPSetup;

impl ZKPSetup {
    /// 执行可信设置并生成proving key和verifying key
    /// 
    /// 注意：这是一个简化的可信设置过程。
    /// 在生产环境中，应该使用多方计算（MPC）来执行可信设置。
    pub fn generate_keys() -> Result<(Vec<u8>, Vec<u8>)> {
        log::info!("🔧 开始ZKP可信设置（Trusted Setup）");
        log::warn!("⚠️  这是简化版可信设置，不适合生产环境");
        log::warn!("⚠️  生产环境应使用多方计算（MPC）进行可信设置");
        
        // 1. 创建空电路用于设置
        let circuit = DIDBindingCircuit::empty();
        
        log::info!("  创建空电路...");
        
        // 2. 执行可信设置
        let mut rng = thread_rng();
        
        log::info!("  生成proving key和verifying key...");
        log::info!("  (这可能需要几秒钟...)");
        
        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
            .map_err(|e| anyhow::anyhow!("可信设置失败: {:?}", e))?;
        
        log::info!("✅ 可信设置完成");
        
        // 3. 序列化keys
        let mut pk_bytes = Vec::new();
        pk.serialize_uncompressed(&mut pk_bytes)
            .context("序列化proving key失败")?;
        
        let mut vk_bytes = Vec::new();
        vk.serialize_uncompressed(&mut vk_bytes)
            .context("序列化verifying key失败")?;
        
        log::info!("  Proving key大小: {} KB", pk_bytes.len() / 1024);
        log::info!("  Verifying key大小: {} bytes", vk_bytes.len());
        
        Ok((pk_bytes, vk_bytes))
    }
    
    /// 保存keys到文件
    pub fn save_keys_to_files(
        pk_bytes: &[u8],
        vk_bytes: &[u8],
        pk_path: &str,
        vk_path: &str,
    ) -> Result<()> {
        log::info!("💾 保存keys到文件");
        
        // 保存proving key
        let pk_file = File::create(pk_path)
            .with_context(|| format!("无法创建文件: {}", pk_path))?;
        let mut pk_writer = BufWriter::new(pk_file);
        
        use std::io::Write;
        pk_writer.write_all(pk_bytes)
            .context("写入proving key失败")?;
        
        log::info!("  ✓ Proving key保存到: {}", pk_path);
        
        // 保存verifying key
        let vk_file = File::create(vk_path)
            .with_context(|| format!("无法创建文件: {}", vk_path))?;
        let mut vk_writer = BufWriter::new(vk_file);
        
        vk_writer.write_all(vk_bytes)
            .context("写入verifying key失败")?;
        
        log::info!("  ✓ Verifying key保存到: {}", vk_path);
        
        Ok(())
    }
    
    /// 从文件加载keys
    pub fn load_keys_from_files(
        pk_path: &str,
        vk_path: &str,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        log::info!("📖 从文件加载keys");
        
        // 读取proving key
        let pk_bytes = std::fs::read(pk_path)
            .with_context(|| format!("无法读取文件: {}", pk_path))?;
        
        log::info!("  ✓ Proving key加载: {} KB", pk_bytes.len() / 1024);
        
        // 读取verifying key
        let vk_bytes = std::fs::read(vk_path)
            .with_context(|| format!("无法读取文件: {}", vk_path))?;
        
        log::info!("  ✓ Verifying key加载: {} bytes", vk_bytes.len());
        
        Ok((pk_bytes, vk_bytes))
    }
    
    /// 验证keys的有效性（通过测试证明生成和验证）
    pub fn verify_keys(pk_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool> {
        use ark_groth16::{ProvingKey, VerifyingKey};
        use std::io::Cursor;
        
        log::info!("🔍 验证keys的有效性");
        
        // 反序列化
        let mut pk_reader = Cursor::new(pk_bytes);
        let pk = ProvingKey::<Bn254>::deserialize_uncompressed(&mut pk_reader)
            .context("反序列化proving key失败")?;
        
        let mut vk_reader = Cursor::new(vk_bytes);
        let vk = VerifyingKey::<Bn254>::deserialize_uncompressed(&mut vk_reader)
            .context("反序列化verifying key失败")?;
        
        log::info!("  生成测试证明...");
        
        // 创建测试电路
        let test_circuit = DIDBindingCircuit::new(
            vec![1u8; 32],  // 测试私钥
            "test_document".to_string(),
            vec![1, 2, 3, 4],  // 测试nonce
            vec![0u8; 32],  // 测试哈希
            vec![2u8; 32],  // 测试公钥
        );
        
        // 生成证明
        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(&pk, test_circuit, &mut rng)
            .map_err(|e| anyhow::anyhow!("生成测试证明失败: {:?}", e))?;
        
        log::info!("  验证测试证明...");
        
        // 验证证明
        let public_inputs = vec![];  // 空公共输入用于测试
        
        let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
            .map_err(|e| anyhow::anyhow!("验证测试证明失败: {:?}", e))?;
        
        if valid {
            log::info!("✅ Keys验证成功");
        } else {
            log::error!("❌ Keys验证失败");
        }
        
        Ok(valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    #[ignore] // 可信设置比较慢，默认忽略
    fn test_generate_keys() {
        env_logger::init();
        
        let result = ZKPSetup::generate_keys();
        assert!(result.is_ok(), "生成keys失败: {:?}", result.err());
        
        let (pk_bytes, vk_bytes) = result.unwrap();
        assert!(!pk_bytes.is_empty());
        assert!(!vk_bytes.is_empty());
        
        println!("✓ Keys生成成功");
    }
    
    #[test]
    #[ignore] // 需要先生成keys
    fn test_save_and_load_keys() {
        let temp_dir = TempDir::new().unwrap();
        let pk_path = temp_dir.path().join("test.pk");
        let vk_path = temp_dir.path().join("test.vk");
        
        // 生成keys
        let (pk_bytes, vk_bytes) = ZKPSetup::generate_keys().unwrap();
        
        // 保存
        ZKPSetup::save_keys_to_files(
            &pk_bytes,
            &vk_bytes,
            pk_path.to_str().unwrap(),
            vk_path.to_str().unwrap(),
        ).unwrap();
        
        // 加载
        let (loaded_pk, loaded_vk) = ZKPSetup::load_keys_from_files(
            pk_path.to_str().unwrap(),
            vk_path.to_str().unwrap(),
        ).unwrap();
        
        assert_eq!(pk_bytes, loaded_pk);
        assert_eq!(vk_bytes, loaded_vk);
        
        println!("✓ 保存和加载测试通过");
    }
}

