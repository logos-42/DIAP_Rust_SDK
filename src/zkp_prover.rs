// DIAP Rust SDK - ZKP证明生成器
// 使用Groth16生成DID-CID绑定证明

use anyhow::{Context, Result};
use ark_bn254::Bn254;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey, PreparedVerifyingKey};
use ark_snark::SNARK;  // 必须导入SNARK trait
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_std::rand::thread_rng;
use crate::zkp_circuit::DIDBindingCircuit;
use ed25519_dalek::SigningKey;

/// ZKP证明生成器
pub struct ZKPProver {
    /// Groth16 proving key
    proving_key: Option<ProvingKey<Bn254>>,
}

/// 证明结果
#[derive(Debug, Clone)]
pub struct ProofResult {
    /// Groth16证明
    pub proof: Vec<u8>,
    
    /// 公共输入
    pub public_inputs: Vec<Vec<u8>>,
    
    /// 生成时间戳
    pub timestamp: String,
}

impl ZKPProver {
    /// 创建新的证明生成器（需要先设置proving key）
    pub fn new() -> Self {
        Self {
            proving_key: None,
        }
    }
    
    /// 设置proving key
    pub fn set_proving_key(&mut self, pk: ProvingKey<Bn254>) {
        self.proving_key = Some(pk);
    }
    
    /// 从文件加载proving key
    pub fn load_proving_key(&mut self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::BufReader;
        
        log::info!("从文件加载proving key: {}", path);
        
        let file = File::open(path)
            .context("无法打开proving key文件")?;
        let mut reader = BufReader::new(file);
        
        let pk = ProvingKey::<Bn254>::deserialize_uncompressed(&mut reader)
            .context("反序列化proving key失败")?;
        
        self.proving_key = Some(pk);
        log::info!("✓ Proving key加载成功");
        
        Ok(())
    }
    
    /// 生成DID-CID绑定证明
    pub fn prove(
        &self,
        secret_key: &SigningKey,
        did_document: &str,
        nonce: &[u8],
        cid_hash: &[u8],
    ) -> Result<ProofResult> {
        log::info!("🔐 开始生成ZKP证明");
        
        let pk = self.proving_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Proving key未设置"))?;
        
        // 1. 从私钥派生公钥
        let verifying_key = secret_key.verifying_key();
        let public_key_bytes = verifying_key.to_bytes();
        
        log::debug!("公共输入:");
        log::debug!("  Nonce长度: {} 字节", nonce.len());
        log::debug!("  CID哈希长度: {} 字节", cid_hash.len());
        log::debug!("  公钥长度: {} 字节", public_key_bytes.len());
        
        // 2. 创建电路实例
        let circuit = DIDBindingCircuit::new(
            secret_key.to_bytes().to_vec(),
            did_document.to_string(),
            nonce.to_vec(),
            cid_hash.to_vec(),
            public_key_bytes.to_vec(),
        );
        
        // 3. 生成证明
        log::info!("生成Groth16证明...");
        let mut rng = thread_rng();
        
        let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng)
            .map_err(|e| anyhow::anyhow!("生成证明失败: {:?}", e))?;
        
        log::info!("✅ 证明生成成功");
        
        // 4. 序列化证明
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes)
            .context("序列化证明失败")?;
        
        log::info!("  证明大小: {} 字节", proof_bytes.len());
        
        Ok(ProofResult {
            proof: proof_bytes,
            public_inputs: vec![
                nonce.to_vec(),
                cid_hash.to_vec(),
                public_key_bytes.to_vec(),
            ],
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// ZKP验证器
pub struct ZKPVerifier {
    /// Groth16 verifying key
    verifying_key: Option<PreparedVerifyingKey<Bn254>>,
}

impl ZKPVerifier {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            verifying_key: None,
        }
    }
    
    /// 设置verifying key
    pub fn set_verifying_key(&mut self, vk: VerifyingKey<Bn254>) {
        self.verifying_key = Some(PreparedVerifyingKey::from(vk));
    }
    
    /// 从文件加载verifying key
    pub fn load_verifying_key(&mut self, path: &str) -> Result<()> {
        use std::fs::File;
        use std::io::BufReader;
        
        log::info!("从文件加载verifying key: {}", path);
        
        let file = File::open(path)
            .context("无法打开verifying key文件")?;
        let mut reader = BufReader::new(file);
        
        let vk = VerifyingKey::<Bn254>::deserialize_uncompressed(&mut reader)
            .context("反序列化verifying key失败")?;
        
        self.verifying_key = Some(PreparedVerifyingKey::from(vk));
        log::info!("✓ Verifying key加载成功");
        
        Ok(())
    }
    
    /// 验证DID-CID绑定证明
    pub fn verify(
        &self,
        proof_bytes: &[u8],
        nonce: &[u8],
        cid_hash: &[u8],
        expected_public_key: &[u8],
    ) -> Result<bool> {
        log::info!("🔍 开始验证ZKP证明");
        
        let pvk = self.verifying_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Verifying key未设置"))?;
        
        // 1. 反序列化证明
        let proof = Proof::<Bn254>::deserialize_uncompressed(proof_bytes)
            .context("反序列化证明失败")?;
        
        // 2. 准备公共输入（转换为Fr元素）
        use ark_ff::Field;
        use ark_bn254::Fr;
        
        let mut public_inputs = Vec::new();
        
        // 将nonce转换为Fr元素
        for chunk in nonce.chunks(31) {  // Fr可以安全容纳31字节
            let mut bytes = [0u8; 32];
            bytes[..chunk.len()].copy_from_slice(chunk);
            if let Some(fr) = Fr::from_random_bytes(&bytes) {
                public_inputs.push(fr);
            }
        }
        
        // 将CID哈希转换为Fr元素
        for chunk in cid_hash.chunks(31) {
            let mut bytes = [0u8; 32];
            bytes[..chunk.len()].copy_from_slice(chunk);
            if let Some(fr) = Fr::from_random_bytes(&bytes) {
                public_inputs.push(fr);
            }
        }
        
        // 将公钥转换为Fr元素
        for chunk in expected_public_key.chunks(31) {
            let mut bytes = [0u8; 32];
            bytes[..chunk.len()].copy_from_slice(chunk);
            if let Some(fr) = Fr::from_random_bytes(&bytes) {
                public_inputs.push(fr);
            }
        }
        
        log::debug!("公共输入元素数量: {}", public_inputs.len());
        
        // 3. 验证证明
        log::info!("验证Groth16证明...");
        
        let is_valid = Groth16::<Bn254>::verify_with_processed_vk(
            pvk,
            &public_inputs,
            &proof,
        ).map_err(|e| anyhow::anyhow!("验证失败: {:?}", e))?;
        
        if is_valid {
            log::info!("✅ 证明验证成功");
        } else {
            log::warn!("⚠️ 证明验证失败（可能是公共输入不匹配或证明无效）");
        }
        
        Ok(is_valid)
    }
}

/// 生成可信设置（仅用于开发测试）
pub fn generate_trusted_setup() -> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> {
    log::warn!("⚠️  生成测试用可信设置（生产环境应使用Powers of Tau）");
    
    // 创建空电路用于设置
    let circuit = DIDBindingCircuit::empty();
    
    // 生成proving key和verifying key
    let mut rng = thread_rng();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("可信设置失败: {:?}", e))?;
    
    log::info!("✅ 可信设置完成");
    
    Ok((pk, vk))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prover_verifier_creation() {
        let _prover = ZKPProver::new();
        let _verifier = ZKPVerifier::new();
        println!("✓ ZKP证明器和验证器创建成功");
    }
    
    #[test]
    #[ignore] // 需要可信设置，耗时较长
    fn test_trusted_setup() {
        let result = generate_trusted_setup();
        assert!(result.is_ok(), "可信设置失败: {:?}", result.err());
        
        let (pk, vk) = result.unwrap();
        println!("✓ 可信设置完成");
        println!("  Proving key大小: {} bytes", 
            ark_serialize::CanonicalSerialize::serialized_size(&pk, ark_serialize::Compress::No));
        println!("  Verifying key大小: {} bytes",
            ark_serialize::CanonicalSerialize::serialized_size(&vk, ark_serialize::Compress::No));
    }
}

