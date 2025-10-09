// ANP Rust SDK - 密钥管理模块
// 负责密钥的生成、存储、加载和导出

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use bs58;
use base64::{Engine as _, engine::general_purpose};

/// 密钥对信息
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// 私钥（32字节）
    pub private_key: [u8; 32],
    
    /// 公钥（32字节）
    pub public_key: [u8; 32],
    
    /// IPNS名称（从公钥派生）
    pub ipns_name: String,
    
    /// DID标识符
    pub did: String,
}

/// 密钥文件格式（用于持久化存储）
#[derive(Debug, Serialize, Deserialize)]
struct KeyFile {
    /// 密钥类型
    key_type: String,
    
    /// 私钥（hex编码）
    private_key: String,
    
    /// 公钥（hex编码）
    public_key: String,
    
    /// IPNS名称
    ipns_name: String,
    
    /// DID
    did: String,
    
    /// 创建时间
    created_at: String,
    
    /// 版本
    version: String,
}

/// 密钥导出格式
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyBackup {
    /// 密钥文件内容（加密）
    encrypted_data: String,
    
    /// 助记词（可选）
    mnemonic: Option<String>,
    
    /// 导出时间
    exported_at: String,
}

impl KeyPair {
    /// 生成新的密钥对
    pub fn generate() -> Result<Self> {
        let mut csprng = OsRng;
        // 生成32字节随机私钥
        let mut secret_bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut csprng, &mut secret_bytes);
        
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        
        let private_key: [u8; 32] = signing_key.to_bytes();
        let public_key: [u8; 32] = verifying_key.to_bytes();
        
        // 从公钥派生IPNS名称
        let ipns_name = Self::derive_ipns_name(&public_key)?;
        
        // 构造DID
        let did = format!("did:ipfs:{}", ipns_name);
        
        Ok(Self {
            private_key,
            public_key,
            ipns_name,
            did,
        })
    }
    
    /// 从私钥加载密钥对
    pub fn from_private_key(private_key: [u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(&private_key);
        let verifying_key = signing_key.verifying_key();
        let public_key: [u8; 32] = verifying_key.to_bytes();
        
        let ipns_name = Self::derive_ipns_name(&public_key)?;
        let did = format!("did:ipfs:{}", ipns_name);
        
        Ok(Self {
            private_key,
            public_key,
            ipns_name,
            did,
        })
    }
    
    /// 从文件加载密钥对
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取密钥文件: {:?}", path))?;
        
        let key_file: KeyFile = serde_json::from_str(&content)
            .with_context(|| format!("无法解析密钥文件: {:?}", path))?;
        
        // 解码私钥
        let private_key_bytes = hex::decode(&key_file.private_key)
            .context("无法解码私钥")?;
        
        if private_key_bytes.len() != 32 {
            anyhow::bail!("私钥长度错误");
        }
        
        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&private_key_bytes);
        
        Self::from_private_key(private_key)
    }
    
    /// 保存密钥对到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建密钥目录: {:?}", parent))?;
        }
        
        let key_file = KeyFile {
            key_type: "Ed25519".to_string(),
            private_key: hex::encode(self.private_key),
            public_key: hex::encode(self.public_key),
            ipns_name: self.ipns_name.clone(),
            did: self.did.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
        };
        
        let content = serde_json::to_string_pretty(&key_file)
            .context("无法序列化密钥")?;
        
        std::fs::write(path, content)
            .with_context(|| format!("无法写入密钥文件: {:?}", path))?;
        
        // 设置文件权限为600（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }
        
        log::info!("密钥已保存到: {:?}", path);
        Ok(())
    }
    
    /// 导出密钥备份
    pub fn export_backup(&self, password: Option<&str>) -> Result<KeyBackup> {
        let key_file = KeyFile {
            key_type: "Ed25519".to_string(),
            private_key: hex::encode(self.private_key),
            public_key: hex::encode(self.public_key),
            ipns_name: self.ipns_name.clone(),
            did: self.did.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
        };
        
        let json_data = serde_json::to_string(&key_file)?;
        
        // 如果提供了密码，加密数据
        let encrypted_data = if let Some(pwd) = password {
            Self::encrypt_data(&json_data, pwd)?
        } else {
            // 无密码时使用base64编码
            general_purpose::STANDARD.encode(json_data)
        };
        
        Ok(KeyBackup {
            encrypted_data,
            mnemonic: None, // TODO: 实现助记词生成
            exported_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// 从备份导入密钥
    pub fn import_from_backup(backup: &KeyBackup, password: Option<&str>) -> Result<Self> {
        // 解密数据
        let json_data = if let Some(pwd) = password {
            Self::decrypt_data(&backup.encrypted_data, pwd)?
        } else {
            String::from_utf8(general_purpose::STANDARD.decode(&backup.encrypted_data)?)?
        };
        
        let key_file: KeyFile = serde_json::from_str(&json_data)?;
        
        let private_key_bytes = hex::decode(&key_file.private_key)?;
        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&private_key_bytes);
        
        Self::from_private_key(private_key)
    }
    
    /// 签名数据
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signing_key = SigningKey::from_bytes(&self.private_key);
        let signature: Signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// 验证签名
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .context("无效的公钥")?;
        
        if signature.len() != 64 {
            return Ok(false);
        }
        
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);
        
        Ok(verifying_key.verify(data, &sig).is_ok())
    }
    
    /// 从公钥派生IPNS名称
    /// 使用libp2p规范：multihash(sha256(public_key))
    fn derive_ipns_name(public_key: &[u8; 32]) -> Result<String> {
        // 1. 计算SHA-256哈希
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();
        
        // 2. 创建multihash（0x12 = sha2-256, 0x20 = 32 bytes）
        let mut multihash = vec![0x12, 0x20];
        multihash.extend_from_slice(&hash);
        
        // 3. Base58编码
        let ipns_name = bs58::encode(&multihash).into_string();
        
        // 4. 添加k51前缀（CIDv1格式）
        // 注意：实际的IPNS名称格式可能需要调整
        Ok(format!("k51qzi5uqu5d{}", &ipns_name[..40]))
    }
    
    /// 加密数据（简单实现，生产环境应使用更强的加密）
    fn encrypt_data(data: &str, _password: &str) -> Result<String> {
        // TODO: 使用AES-GCM加密
        // 当前简单实现：使用base64编码
        Ok(general_purpose::STANDARD.encode(data))
    }
    
    /// 解密数据
    fn decrypt_data(encrypted: &str, _password: &str) -> Result<String> {
        // TODO: 使用AES-GCM解密
        // 当前简单实现：使用base64解码
        let bytes = general_purpose::STANDARD.decode(encrypted)?;
        Ok(String::from_utf8(bytes)?)
    }
}

/// 密钥管理器
pub struct KeyManager {
    #[allow(dead_code)]
    config_dir: PathBuf,
}

impl KeyManager {
    /// 创建新的密钥管理器
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }
    
    /// 加载或生成密钥
    pub fn load_or_generate(&self, key_path: &PathBuf) -> Result<KeyPair> {
        if key_path.exists() {
            log::info!("从文件加载密钥: {:?}", key_path);
            KeyPair::from_file(key_path)
        } else {
            log::info!("生成新密钥");
            let keypair = KeyPair::generate()?;
            keypair.save_to_file(key_path)?;
            Ok(keypair)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_generate_keypair() {
        let keypair = KeyPair::generate().unwrap();
        assert_eq!(keypair.private_key.len(), 32);
        assert_eq!(keypair.public_key.len(), 32);
        assert!(keypair.did.starts_with("did:ipfs:"));
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate().unwrap();
        let data = b"test message";
        
        let signature = keypair.sign(data).unwrap();
        assert!(keypair.verify(data, &signature).unwrap());
        
        // 验证错误的数据
        let wrong_data = b"wrong message";
        assert!(!keypair.verify(wrong_data, &signature).unwrap());
    }
    
    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("test.key");
        
        let keypair1 = KeyPair::generate().unwrap();
        keypair1.save_to_file(&key_path).unwrap();
        
        let keypair2 = KeyPair::from_file(&key_path).unwrap();
        assert_eq!(keypair1.private_key, keypair2.private_key);
        assert_eq!(keypair1.did, keypair2.did);
    }
}
