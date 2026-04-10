// DIAP Rust SDK - 实名认证模块
// 支持身份证绑定、用户 DID 身份锚定、智能体签名授权

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 实名认证凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealNameCredential {
    /// 凭证 ID（唯一标识）
    pub credential_id: String,

    /// 用户 DID（实名认证后获得的 DID）
    pub user_did: String,

    /// 加密的身份证号（Base64 编码的 AES-256-GCM 密文）
    pub encrypted_id_number: String,

    /// 加密的姓名（Base64 编码的 AES-256-GCM 密文）
    pub encrypted_name: String,

    /// 认证时间
    pub auth_time: String,

    /// 认证机构（可选）
    pub auth_authority: Option<String>,

    /// 认证级别
    pub auth_level: AuthLevel,
}

/// 认证级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthLevel {
    /// 基础认证（仅手机号等）
    Basic,
    /// 中级认证（身份证绑定）
    Medium,
    /// 高级认证（人脸识别+身份证）
    High,
}

/// 用户身份信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// 用户 DID
    pub did: String,

    /// 关联的实名凭证 ID
    pub credential_id: String,

    /// 用户类型
    pub user_type: UserType,

    /// 创建时间
    pub created_at: String,

    /// 用户公钥（Base64）
    pub public_key: String,
}

/// 用户类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserType {
    /// 实名用户
    RealName,
    /// 匿名用户
    Anonymous,
    /// 组织用户
    Organization,
}

/// 智能体授权信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuthorization {
    /// 授权 ID
    pub authorization_id: String,

    /// 授权者 DID（用户 DID）
    pub authorizer_did: String,

    /// 被授权的智能体 DID
    pub agent_did: String,

    /// 授权时间
    pub authorized_at: String,

    /// 授权有效期（可选）
    pub expires_at: Option<String>,

    /// 授权级别
    pub auth_level: AgentAuthLevel,

    /// 授权范围（可为空表示全部权限）
    pub scope: Option<Vec<String>>,

    /// 授权签名（用户 DID 对智能体 DID 的签名）
    pub signature: String,
}

/// 智能体授权级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAuthLevel {
    /// 只读权限
    ReadOnly,
    /// 读写权限
    ReadWrite,
    /// 管理员权限
    Admin,
    /// 完全控制
    FullControl,
}

/// 实名认证管理器
pub struct RealNameAuthManager {
    /// 当前用户的密钥对（用于签名）
    keypair: Option<crate::key_manager::KeyPair>,
}

/// 签名数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSignature {
    /// 签名者 DID
    pub signer_did: String,

    /// 被签名的智能体 DID
    pub agent_did: String,

    /// 签名（Base64）
    pub signature: String,

    /// 签名时间
    pub signed_at: String,

    /// 签名版本
    pub version: String,
}

impl RealNameAuthManager {
    /// 创建实名认证管理器
    pub fn new() -> Self {
        Self { keypair: None }
    }

    /// 设置密钥对
    pub fn with_keypair(mut self, keypair: crate::key_manager::KeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    /// 创建用户 DID（基于实名认证）
    pub fn create_user_did(&self, keypair: &crate::key_manager::KeyPair) -> String {
        keypair.did.clone()
    }

    /// 生成实名认证凭证
    pub fn create_credential(
        &self,
        keypair: &crate::key_manager::KeyPair,
        id_number: &str,
        name: &str,
        auth_level: AuthLevel,
    ) -> Result<RealNameCredential> {
        let signing_key = SigningKey::from_bytes(&keypair.private_key);

        // 加密身份证号
        let encrypted_id = self.encrypt_personal_info(&signing_key, id_number)?;

        // 加密姓名
        let encrypted_name = self.encrypt_personal_info(&signing_key, name)?;

        // 生成凭证 ID
        let mut hasher = Sha256::new();
        hasher.update(keypair.public_key.as_slice());
        hasher.update(id_number.as_bytes());
        let credential_id = hex::encode(hasher.finalize());

        Ok(RealNameCredential {
            credential_id,
            user_did: keypair.did.clone(),
            encrypted_id_number: encrypted_id,
            encrypted_name,
            auth_time: chrono::Utc::now().to_rfc3339(),
            auth_authority: None,
            auth_level,
        })
    }

    /// 对智能体进行签名授权
    pub fn authorize_agent(
        &self,
        authorizer_keypair: &crate::key_manager::KeyPair,
        agent_did: &str,
        auth_level: AgentAuthLevel,
        scope: Option<Vec<String>>,
        expires_at: Option<String>,
    ) -> Result<AgentAuthorization> {
        let signing_key = SigningKey::from_bytes(&authorizer_keypair.private_key);

        // 创建授权数据
        let auth_data = format!(
            "{}|{}|{}|{}",
            authorizer_keypair.did,
            agent_did,
            chrono::Utc::now().timestamp(),
            match auth_level {
                AgentAuthLevel::ReadOnly => "read",
                AgentAuthLevel::ReadWrite => "write",
                AgentAuthLevel::Admin => "admin",
                AgentAuthLevel::FullControl => "full",
            }
        );

        // 签名
        let signature = signing_key.sign(auth_data.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());

        // 生成授权 ID
        let mut hasher = Sha256::new();
        hasher.update(authorizer_keypair.did.as_bytes());
        hasher.update(agent_did.as_bytes());
        let authorization_id = hex::encode(hasher.finalize());

        Ok(AgentAuthorization {
            authorization_id,
            authorizer_did: authorizer_keypair.did.clone(),
            agent_did: agent_did.to_string(),
            authorized_at: chrono::Utc::now().to_rfc3339(),
            expires_at,
            auth_level,
            scope,
            signature: signature_b64,
        })
    }

    /// 验证智能体授权签名
    pub fn verify_agent_authorization(
        &self,
        authorization: &AgentAuthorization,
        authorizer_public_key: &[u8],
    ) -> Result<bool> {
        let verifying_key =
            VerifyingKey::from_bytes(authorizer_public_key.try_into().context("公钥长度错误")?)?;

        // 重建授权数据
        let _auth_data = format!(
            "{}|{}|{}|{}",
            authorization.authorizer_did,
            authorization.agent_did,
            chrono::Utc::now().timestamp(), // 注意：这里需要用实际时间
            match authorization.auth_level {
                AgentAuthLevel::ReadOnly => "read",
                AgentAuthLevel::ReadWrite => "write",
                AgentAuthLevel::Admin => "admin",
                AgentAuthLevel::FullControl => "full",
            }
        );

        // 解码签名
        let signature_bytes = general_purpose::STANDARD
            .decode(&authorization.signature)
            .context("解码签名失败")?;

        let signature = Signature::from_slice(&signature_bytes).context("解析签名失败")?;

        // 验证签名（使用当前时间戳的简化验证）
        let now = chrono::Utc::now().timestamp();
        let auth_time = chrono::DateTime::parse_from_rfc3339(&authorization.authorized_at)
            .map(|dt| dt.timestamp())
            .unwrap_or(now);

        let auth_data_for_verify = format!(
            "{}|{}|{}|{}",
            authorization.authorizer_did,
            authorization.agent_did,
            auth_time,
            match authorization.auth_level {
                AgentAuthLevel::ReadOnly => "read",
                AgentAuthLevel::ReadWrite => "write",
                AgentAuthLevel::Admin => "admin",
                AgentAuthLevel::FullControl => "full",
            }
        );

        Ok(verifying_key
            .verify(auth_data_for_verify.as_bytes(), &signature)
            .is_ok())
    }

    /// 创建智能体签名（用于智能体创建时的授权）
    pub fn sign_agent_creation(
        &self,
        signer_keypair: &crate::key_manager::KeyPair,
        agent_metadata: &AgentMetadata,
    ) -> Result<AgentSignature> {
        let signing_key = SigningKey::from_bytes(&signer_keypair.private_key);

        // 创建签名数据
        let sign_data = serde_json::to_string(agent_metadata).context("序列化智能体元数据失败")?;

        // 签名
        let signature = signing_key.sign(sign_data.as_bytes());
        let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());

        Ok(AgentSignature {
            signer_did: signer_keypair.did.clone(),
            agent_did: agent_metadata.agent_did.clone(),
            signature: signature_b64,
            signed_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
        })
    }

    /// 验证智能体创建签名
    pub fn verify_agent_signature(
        &self,
        agent_signature: &AgentSignature,
        signer_public_key: &[u8],
        agent_metadata: &AgentMetadata,
    ) -> Result<bool> {
        let verifying_key =
            VerifyingKey::from_bytes(signer_public_key.try_into().context("公钥长度错误")?)?;

        // 重建签名数据
        let sign_data = serde_json::to_string(agent_metadata).context("序列化智能体元数据失败")?;

        // 解码签名
        let signature_bytes = general_purpose::STANDARD
            .decode(&agent_signature.signature)
            .context("解码签名失败")?;

        let signature = Signature::from_slice(&signature_bytes).context("解析签名失败")?;

        Ok(verifying_key
            .verify(sign_data.as_bytes(), &signature)
            .is_ok())
    }

    /// 加密个人信息
    fn encrypt_personal_info(&self, signing_key: &SigningKey, data: &str) -> Result<String> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        // 从私钥派生 AES 密钥（简化版，实际应使用 KDF）
        let mut key_bytes = [0u8; 32];
        let key_hash = Sha256::digest(signing_key.to_bytes());
        key_bytes.copy_from_slice(&key_hash[..32]);

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // 生成随机 nonce
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(|e| anyhow::anyhow!("加密失败: {:?}", e))?;

        // 组合 nonce + ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);

        Ok(general_purpose::STANDARD.encode(result))
    }

    /// 解密个人信息
    pub fn decrypt_personal_info(
        &self,
        keypair: &crate::key_manager::KeyPair,
        encrypted: &str,
    ) -> Result<String> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Key, Nonce};

        let signing_key = SigningKey::from_bytes(&keypair.private_key);

        // 派生 AES 密钥
        let mut key_bytes = [0u8; 32];
        let key_hash = Sha256::digest(signing_key.to_bytes());
        key_bytes.copy_from_slice(&key_hash[..32]);

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // 解码
        let data = general_purpose::STANDARD
            .decode(encrypted)
            .context("解码失败")?;

        if data.len() < 12 {
            anyhow::bail!("数据太短");
        }

        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        // 解密
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("解密失败: {:?}", e))?;

        String::from_utf8(plaintext).context("UTF-8 解码失败")
    }
}

/// 智能体元数据（用于签名）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// 智能体 DID
    pub agent_did: String,

    /// 智能体名称
    pub name: String,

    /// 智能体类型
    pub agent_type: String,

    /// 创建时间
    pub created_at: String,

    /// 智能体公钥
    pub public_key: String,

    /// 附加数据（可选）
    pub extra: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

/// 授权链（用于验证智能体的授权来源）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationChain {
    /// 链 ID
    pub chain_id: String,

    /// 根授权者（用户 DID）
    pub root_authorizer: String,

    /// 授权路径
    pub authorization_path: Vec<AgentAuthorization>,

    /// 链创建时间
    pub created_at: String,
}

impl Default for RealNameAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_did() {
        let manager = RealNameAuthManager::new();
        let keypair = crate::key_manager::KeyPair::generate().unwrap();

        let user_did = manager.create_user_did(&keypair);
        assert!(user_did.starts_with("did:key:"));

        println!("用户 DID: {}", user_did);
    }

    #[test]
    fn test_authorize_agent() {
        let manager = RealNameAuthManager::new();

        // 用户密钥对
        let user_keypair = crate::key_manager::KeyPair::generate().unwrap();

        // 智能体 DID
        let agent_did = "did:key:zabc123";

        // 授权智能体
        let auth = manager
            .authorize_agent(
                &user_keypair,
                agent_did,
                AgentAuthLevel::FullControl,
                None,
                None,
            )
            .unwrap();

        println!("授权 ID: {}", auth.authorization_id);
        println!("授权者: {}", auth.authorizer_did);
        println!("智能体: {}", auth.agent_did);

        // 验证签名
        let is_valid = manager
            .verify_agent_authorization(&auth, &user_keypair.public_key)
            .unwrap();

        assert!(is_valid);
    }
}
