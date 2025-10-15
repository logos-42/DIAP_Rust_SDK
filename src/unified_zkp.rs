// DIAP Rust SDK - 统一ZKP接口
// 解决Noir和Arkworks之间的功能错位问题

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// ZKP方案类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZKPScheme {
    /// Noir ZKP方案
    Noir,
    /// Arkworks Groth16方案
    Arkworks,
}

/// 统一的ZKP输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedZKPInputs {
    /// 私钥（字节数组）
    pub secret_key: Vec<u8>,
    /// DID文档内容
    pub did_document: String,
    /// 挑战nonce
    pub nonce: Vec<u8>,
    /// CID哈希
    pub cid_hash: Vec<u8>,
    /// 期望的公钥
    pub expected_public_key: Vec<u8>,
}

/// 统一的ZKP输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedZKPOutput {
    /// 生成的证明
    pub proof: Vec<u8>,
    /// 公共输入
    pub public_inputs: Vec<u8>,
    /// 电路输出
    pub circuit_output: String,
    /// 使用的ZKP方案
    pub scheme: ZKPScheme,
    /// 生成时间戳
    pub timestamp: String,
    /// 生成耗时（毫秒）
    pub generation_time_ms: u64,
}

/// 统一的ZKP验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedVerificationResult {
    /// 验证是否成功
    pub is_valid: bool,
    /// 验证耗时（毫秒）
    pub verification_time_ms: u64,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
    /// 使用的ZKP方案
    pub scheme: ZKPScheme,
}

/// 统一ZKP管理器
pub struct UnifiedZKPManager {
    /// 当前使用的ZKP方案
    scheme: ZKPScheme,
    /// Noir电路路径
    noir_circuits_path: String,
    /// Arkworks密钥路径
    arkworks_keys_path: String,
}

impl UnifiedZKPManager {
    /// 创建新的统一ZKP管理器
    pub fn new(
        scheme: ZKPScheme,
        noir_circuits_path: String,
        arkworks_keys_path: String,
    ) -> Self {
        Self {
            scheme,
            noir_circuits_path,
            arkworks_keys_path,
        }
    }

    /// 生成ZKP证明
    pub async fn generate_proof(
        &mut self,
        inputs: &UnifiedZKPInputs,
    ) -> Result<UnifiedZKPOutput> {
        let start_time = std::time::Instant::now();
        
        log::info!("🔐 使用{:?}方案生成ZKP证明", self.scheme);
        
        match self.scheme {
            ZKPScheme::Noir => {
                self.generate_noir_proof(inputs).await
            }
            ZKPScheme::Arkworks => {
                self.generate_arkworks_proof(inputs).await
            }
        }
    }

    /// 验证ZKP证明
    pub async fn verify_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        circuit_output: &str,
    ) -> Result<UnifiedVerificationResult> {
        let start_time = std::time::Instant::now();
        
        log::info!("🔍 使用{:?}方案验证ZKP证明", self.scheme);
        
        match self.scheme {
            ZKPScheme::Noir => {
                self.verify_noir_proof(proof, public_inputs, circuit_output).await
            }
            ZKPScheme::Arkworks => {
                self.verify_arkworks_proof(proof, public_inputs, circuit_output).await
            }
        }
    }

    /// 切换ZKP方案
    pub fn switch_scheme(&mut self, new_scheme: ZKPScheme) {
        log::info!("🔄 切换ZKP方案: {:?} -> {:?}", self.scheme, new_scheme);
        self.scheme = new_scheme;
    }

    /// 获取当前方案
    pub fn get_current_scheme(&self) -> &ZKPScheme {
        &self.scheme
    }

    // 私有方法：生成Noir证明
    async fn generate_noir_proof(
        &self,
        inputs: &UnifiedZKPInputs,
    ) -> Result<UnifiedZKPOutput> {
        use crate::noir_zkp::NoirZKPManager;
        use crate::KeyPair;
        use crate::DIDDocument;
        
        let start_time = std::time::Instant::now();
        
        // 创建Noir ZKP管理器
        let mut noir_manager = NoirZKPManager::new(self.noir_circuits_path.clone());
        
        // 创建KeyPair（从私钥）
        if inputs.secret_key.len() != 32 {
            anyhow::bail!("私钥长度必须是32字节");
        }
        let mut secret_key_array = [0u8; 32];
        secret_key_array.copy_from_slice(&inputs.secret_key[..32]);
        let keypair = KeyPair::from_private_key(secret_key_array)?;
        
        // 创建DID文档
        let did_document = DIDDocument {
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            id: keypair.did.clone(),
            verification_method: vec![],
            authentication: vec![],
            service: None,
            created: chrono::Utc::now().to_rfc3339(),
        };
        
        // 生成证明
        let noir_result = noir_manager.generate_did_binding_proof(
            &keypair,
            &did_document,
            &inputs.cid_hash,
            &inputs.nonce,
        ).await?;
        
        let generation_time = start_time.elapsed().as_millis() as u64;
        
        Ok(UnifiedZKPOutput {
            proof: noir_result.proof,
            public_inputs: noir_result.public_inputs,
            circuit_output: noir_result.circuit_output,
            scheme: ZKPScheme::Noir,
            timestamp: chrono::Utc::now().to_rfc3339(),
            generation_time_ms: generation_time,
        })
    }

    // 私有方法：生成Arkworks证明
    async fn generate_arkworks_proof(
        &self,
        inputs: &UnifiedZKPInputs,
    ) -> Result<UnifiedZKPOutput> {
        use crate::zkp_prover::ZKPProver;
        use crate::zkp_setup::ZKPSetup;
        use ed25519_dalek::SigningKey;
        
        let start_time = std::time::Instant::now();
        
        // 创建证明器
        let mut prover = ZKPProver::new();
        
        // 加载或生成密钥
        let (pk, _vk) = ZKPSetup::generate_keys()?;
        // 注意：这里需要反序列化pk_bytes到ProvingKey
        // 为了简化，我们使用generate_trusted_setup函数
        let (pk_real, _vk_real) = crate::zkp_prover::generate_trusted_setup()?;
        prover.set_proving_key(pk_real);
        
        // 创建签名密钥
        let signing_key = SigningKey::from_bytes(&inputs.secret_key[..32].try_into()?);
        
        // 生成证明
        let arkworks_result = prover.prove(
            &signing_key,
            &inputs.did_document,
            &inputs.nonce,
            &inputs.cid_hash,
        )?;
        
        let generation_time = start_time.elapsed().as_millis() as u64;
        
        Ok(UnifiedZKPOutput {
            proof: arkworks_result.proof,
            public_inputs: serde_json::to_vec(&arkworks_result.public_inputs)?,
            circuit_output: "arkworks_output".to_string(), // Arkworks没有电路输出概念
            scheme: ZKPScheme::Arkworks,
            timestamp: arkworks_result.timestamp,
            generation_time_ms: generation_time,
        })
    }

    // 私有方法：验证Noir证明
    async fn verify_noir_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        circuit_output: &str,
    ) -> Result<UnifiedVerificationResult> {
        use crate::noir_verifier::ImprovedNoirZKPManager;
        
        let start_time = std::time::Instant::now();
        
        let verifier = ImprovedNoirZKPManager::new(self.noir_circuits_path.clone());
        let result = verifier.verify_proof(proof, public_inputs, circuit_output).await?;
        
        let verification_time = start_time.elapsed().as_millis() as u64;
        
        Ok(UnifiedVerificationResult {
            is_valid: result.is_valid,
            verification_time_ms: verification_time,
            error_message: result.error_message,
            scheme: ZKPScheme::Noir,
        })
    }

    // 私有方法：验证Arkworks证明
    async fn verify_arkworks_proof(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        _circuit_output: &str,
    ) -> Result<UnifiedVerificationResult> {
        use crate::zkp_prover::ZKPVerifier;
        
        let start_time = std::time::Instant::now();
        
        // 创建验证器
        let mut verifier = ZKPVerifier::new();
        
        // 生成验证密钥
        let (_pk_real, vk_real) = crate::zkp_prover::generate_trusted_setup()?;
        verifier.set_verifying_key(vk_real);
        
        // 解析公共输入
        let inputs: Vec<Vec<u8>> = serde_json::from_slice(public_inputs)?;
        if inputs.len() < 3 {
            anyhow::bail!("公共输入格式错误");
        }
        
        // 验证证明
        let is_valid = verifier.verify(
            proof,
            &inputs[0], // nonce
            &inputs[1], // cid_hash
            &inputs[2], // public_key
        )?;
        
        let verification_time = start_time.elapsed().as_millis() as u64;
        
        Ok(UnifiedVerificationResult {
            is_valid,
            verification_time_ms: verification_time,
            error_message: if is_valid { None } else { Some("Arkworks验证失败".to_string()) },
            scheme: ZKPScheme::Arkworks,
        })
    }
}

/// 性能对比测试器
pub struct ZKPPerformanceTester {
    noir_manager: UnifiedZKPManager,
    arkworks_manager: UnifiedZKPManager,
}

impl ZKPPerformanceTester {
    /// 创建新的性能测试器
    pub fn new(
        noir_circuits_path: String,
        arkworks_keys_path: String,
    ) -> Self {
        Self {
            noir_manager: UnifiedZKPManager::new(
                ZKPScheme::Noir,
                noir_circuits_path.clone(),
                arkworks_keys_path.clone(),
            ),
            arkworks_manager: UnifiedZKPManager::new(
                ZKPScheme::Arkworks,
                noir_circuits_path,
                arkworks_keys_path,
            ),
        }
    }

    /// 运行性能对比测试
    pub async fn run_performance_test(
        &mut self,
        inputs: &UnifiedZKPInputs,
    ) -> Result<ZKPPerformanceComparison> {
        log::info!("🚀 开始ZKP性能对比测试");
        
        // 测试Noir方案
        let noir_start = std::time::Instant::now();
        let noir_output = self.noir_manager.generate_proof(inputs).await?;
        let noir_generation_time = noir_start.elapsed();
        
        let noir_verify_start = std::time::Instant::now();
        let noir_verify_result = self.noir_manager.verify_proof(
            &noir_output.proof,
            &noir_output.public_inputs,
            &noir_output.circuit_output,
        ).await?;
        let noir_verification_time = noir_verify_start.elapsed();
        
        // 测试Arkworks方案
        let arkworks_start = std::time::Instant::now();
        let arkworks_output = self.arkworks_manager.generate_proof(inputs).await?;
        let arkworks_generation_time = arkworks_start.elapsed();
        
        let arkworks_verify_start = std::time::Instant::now();
        let arkworks_verify_result = self.arkworks_manager.verify_proof(
            &arkworks_output.proof,
            &arkworks_output.public_inputs,
            &arkworks_output.circuit_output,
        ).await?;
        let arkworks_verification_time = arkworks_verify_start.elapsed();
        
        Ok(ZKPPerformanceComparison {
            noir_results: ZKPSchemeResults {
                generation_time_ms: noir_generation_time.as_millis() as u64,
                verification_time_ms: noir_verification_time.as_millis() as u64,
                proof_size_bytes: noir_output.proof.len(),
                public_inputs_size_bytes: noir_output.public_inputs.len(),
                verification_success: noir_verify_result.is_valid,
            },
            arkworks_results: ZKPSchemeResults {
                generation_time_ms: arkworks_generation_time.as_millis() as u64,
                verification_time_ms: arkworks_verification_time.as_millis() as u64,
                proof_size_bytes: arkworks_output.proof.len(),
                public_inputs_size_bytes: arkworks_output.public_inputs.len(),
                verification_success: arkworks_verify_result.is_valid,
            },
        })
    }
}

/// ZKP方案性能结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKPSchemeResults {
    pub generation_time_ms: u64,
    pub verification_time_ms: u64,
    pub proof_size_bytes: usize,
    pub public_inputs_size_bytes: usize,
    pub verification_success: bool,
}

/// ZKP性能对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKPPerformanceComparison {
    pub noir_results: ZKPSchemeResults,
    pub arkworks_results: ZKPSchemeResults,
}

impl ZKPPerformanceComparison {
    /// 打印性能对比报告
    pub fn print_comparison_report(&self) {
        println!("\n📊 ZKP性能对比报告");
        println!("==========================================");
        
        println!("\n🔹 Noir方案:");
        println!("   证明生成时间: {}ms", self.noir_results.generation_time_ms);
        println!("   证明验证时间: {}ms", self.noir_results.verification_time_ms);
        println!("   证明大小: {} bytes", self.noir_results.proof_size_bytes);
        println!("   公共输入大小: {} bytes", self.noir_results.public_inputs_size_bytes);
        println!("   验证结果: {}", if self.noir_results.verification_success { "✅ 成功" } else { "❌ 失败" });
        
        println!("\n🔸 Arkworks方案:");
        println!("   证明生成时间: {}ms", self.arkworks_results.generation_time_ms);
        println!("   证明验证时间: {}ms", self.arkworks_results.verification_time_ms);
        println!("   证明大小: {} bytes", self.arkworks_results.proof_size_bytes);
        println!("   公共输入大小: {} bytes", self.arkworks_results.public_inputs_size_bytes);
        println!("   验证结果: {}", if self.arkworks_results.verification_success { "✅ 成功" } else { "❌ 失败" });
        
        println!("\n📈 性能对比:");
        let gen_ratio = self.noir_results.generation_time_ms as f64 / self.arkworks_results.generation_time_ms as f64;
        let ver_ratio = self.noir_results.verification_time_ms as f64 / self.arkworks_results.verification_time_ms as f64;
        
        println!("   生成时间比例 (Noir/Arkworks): {:.2}x", gen_ratio);
        println!("   验证时间比例 (Noir/Arkworks): {:.2}x", ver_ratio);
        
        if gen_ratio < 1.0 {
            println!("   🏆 Noir在证明生成上更快");
        } else {
            println!("   🏆 Arkworks在证明生成上更快");
        }
        
        if ver_ratio < 1.0 {
            println!("   🏆 Noir在证明验证上更快");
        } else {
            println!("   🏆 Arkworks在证明验证上更快");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unified_zkp_manager() {
        let manager = UnifiedZKPManager::new(
            ZKPScheme::Noir,
            "./noir_circuits".to_string(),
            "./arkworks_keys".to_string(),
        );
        
        assert_eq!(*manager.get_current_scheme(), ZKPScheme::Noir);
    }
}
