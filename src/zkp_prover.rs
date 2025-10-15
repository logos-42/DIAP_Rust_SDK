// DIAP Rust SDK - ZKP证明生成器
// 使用Groth16生成DID-CID绑定证明

use anyhow::{Context, Result};
use ark_bn254::Bn254;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey, PreparedVerifyingKey};
use ark_snark::SNARK;  // 必须导入SNARK trait
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_std::rand::thread_rng;
use serde::{Serialize, Deserialize};
use crate::zkp_circuit::DIDBindingCircuit;
use ed25519_dalek::SigningKey;

/// ZKP证明生成器
pub struct ZKPProver {
    /// Groth16 proving key
    proving_key: Option<ProvingKey<Bn254>>,
}

/// 证明结果
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    
    /// 验证DID-CID绑定证明（改进版：与电路公共输入一致）
    pub fn verify(
        &self,
        proof_bytes: &[u8],
        nonce: &[u8],
        cid_hash: &[u8],
        expected_public_key: &[u8],
    ) -> Result<bool> {
        log::info!("🔍 开始验证ZKP证明（改进版）");
        
        let pvk = self.verifying_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Verifying key未设置"))?;
        
        // 1. 反序列化证明
        let proof = Proof::<Bn254>::deserialize_uncompressed(proof_bytes)
            .context("反序列化证明失败")?;
        
        // 2. 准备公共输入（与电路构造保持一致）
        
        let mut public_inputs = Vec::new();
        
        // 公共输入顺序（必须与电路中的new_input调用顺序一致）：
        // 1. expected_did_hash_fields (Vec<Fr>)
        // 2. public_key_hash (Fr)
        // 3. nonce_hash (Fr)
        
        // 1) 将CID哈希转换为Fr元素数组
        let cid_hash_fields = Self::bytes_to_field_elements(cid_hash);
        public_inputs.extend(cid_hash_fields);
        
        // 2) 计算公钥哈希并转换为单个Fr元素
        use blake2::{Blake2s256, Digest};
        let pk_hash_bytes = Blake2s256::digest(expected_public_key);
        let pk_hash_field = Self::bytes_to_single_field(&pk_hash_bytes);
        public_inputs.push(pk_hash_field);
        
        // 3) 计算nonce哈希并转换为单个Fr元素
        let nonce_hash_bytes = Blake2s256::digest(nonce);
        let nonce_hash_field = Self::bytes_to_single_field(&nonce_hash_bytes);
        public_inputs.push(nonce_hash_field);
        
        log::debug!("公共输入元素数量: {}", public_inputs.len());
        log::debug!("  CID哈希字段: {} 个", public_inputs.len() - 2);
        log::debug!("  公钥哈希: 1 个");
        log::debug!("  Nonce哈希: 1 个");
        
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
    
    /// 将字节数组转换为字段元素数组（与电路保持一致）
    fn bytes_to_field_elements(bytes: &[u8]) -> Vec<ark_bn254::Fr> {
        use ark_ff::PrimeField;
        bytes.chunks(31) // Fr字段最多安全编码31字节
            .map(|chunk| {
                let mut bytes_padded = [0u8; 32];
                bytes_padded[..chunk.len()].copy_from_slice(chunk);
                ark_bn254::Fr::from_le_bytes_mod_order(&bytes_padded)
            })
            .collect()
    }
    
    /// 将字节数组压缩为单个字段元素（与电路保持一致）
    fn bytes_to_single_field(bytes: &[u8]) -> ark_bn254::Fr {
        use ark_ff::PrimeField;
        let len = bytes.len().min(31);
        let mut bytes_padded = [0u8; 32];
        bytes_padded[..len].copy_from_slice(&bytes[..len]);
        ark_bn254::Fr::from_le_bytes_mod_order(&bytes_padded)
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

