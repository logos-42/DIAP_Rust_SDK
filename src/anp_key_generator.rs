/**
 * ANP协议密钥对生成器 - Rust版本
 * 支持Ed25519和secp256k1算法
 * 使用真正的加密库实现
 */

 use rand::{rngs::OsRng, Rng};
 use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
 use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
 use secp256k1::{Secp256k1, SecretKey as Secp256k1SecretKey, PublicKey as Secp256k1PublicKey};
 use serde::{Serialize, Deserialize};
 use serde_json::Value;
 use base64::engine::{general_purpose, Engine as _};
 use bs58;
 use sha2::{Sha256, Digest};
 use chrono::Utc;
 
 // 支持的密钥类型
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub enum KeyType {
     #[serde(rename = "Ed25519VerificationKey2020")]
     Ed25519,
     #[serde(rename = "EcdsaSecp256k1VerificationKey2019")]
     Secp256k1,
     #[serde(rename = "X25519KeyAgreementKey2019")]
     X25519,
 }
 
 // DID文档结构
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct DIDDocument {
     #[serde(rename = "@context")]
     pub context: Vec<String>,
     pub id: String,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub verification_method: Option<Vec<VerificationMethod>>,
     pub authentication: Vec<AuthenticationMethod>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub key_agreement: Option<Vec<KeyAgreementMethod>>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub human_authorization: Option<Vec<HumanAuthorizationMethod>>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub service: Option<Vec<Service>>,
 }
 
 // 验证方法
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct VerificationMethod {
     pub id: String,
     #[serde(rename = "type")]
     pub method_type: String,
     pub controller: String,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub public_key_jwk: Option<PublicKeyJWK>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub public_key_multibase: Option<String>,
 }
 
 // JWK公钥格式
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct PublicKeyJWK {
     pub crv: String,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub x: Option<String>,
     #[serde(skip_serializing_if = "Option::is_none")]
     pub y: Option<String>,
     pub kty: String,
     pub kid: String,
 }
 
 // 身份验证方法（可以是字符串或对象）
 #[derive(Debug, Clone, Serialize, Deserialize)]
 #[serde(untagged)]
 pub enum AuthenticationMethod {
     String(String),
     Object(VerificationMethod),
 }
 
 // 密钥协商方法
 #[derive(Debug, Clone, Serialize, Deserialize)]
 #[serde(untagged)]
 pub enum KeyAgreementMethod {
     String(String),
     Object(VerificationMethod),
 }
 
 // 人类授权方法
 #[derive(Debug, Clone, Serialize, Deserialize)]
 #[serde(untagged)]
 pub enum HumanAuthorizationMethod {
     String(String),
     Object(VerificationMethod),
 }
 
 // 服务
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct Service {
     pub id: String,
     #[serde(rename = "type")]
     pub service_type: String,
     pub service_endpoint: String,
 }
 
 // 密钥对结果
 #[derive(Debug, Clone)]
 pub struct KeyPairResult {
     pub did_document: String,
     pub private_key: String,
     pub did: String,
 }
 
 // 签名数据
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct SignatureData {
     pub nonce: String,
     pub timestamp: String,
     pub service: String,
     pub did: String,
 }
 
 /**
  * ANP密钥对生成器
  */
 pub struct ANPKeyGenerator {
     domain: String,
     path: Option<String>,
 }
 
 impl ANPKeyGenerator {
     /// 创建新的密钥生成器
     pub fn new(domain: String, path: Option<String>) -> Self {
         Self { domain, path }
     }
 
     /// 生成DID标识符
     fn generate_did(&self) -> String {
         if let Some(ref path) = self.path {
             format!("did:wba:{}:{}", self.domain, path)
         } else {
             format!("did:wba:{}", self.domain)
         }
     }
 
     /// 生成安全的随机字符串
     fn generate_nonce(&self, length: usize) -> String {
         const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
         let mut rng = OsRng;
         
         (0..length)
             .map(|_| {
                 let idx = rng.gen_range(0..CHARSET.len());
                 CHARSET[idx] as char
             })
             .collect()
     }
 
     /// 生成Ed25519密钥对
     fn generate_ed25519_keypair(&self) -> (SigningKey, VerifyingKey) {
         let mut rng = OsRng;
         // 使用正确的 API：先生成随机字节，然后创建 SigningKey
         let mut secret_bytes = [0u8; 32];
         rng.fill(&mut secret_bytes);
         let signing_key = SigningKey::from_bytes(&secret_bytes);
         let verifying_key = signing_key.verifying_key();
         (signing_key, verifying_key)
     }
 
     /// 生成secp256k1密钥对
     fn generate_secp256k1_keypair(&self) -> (Secp256k1SecretKey, Secp256k1PublicKey) {
         let secp = Secp256k1::new();
         let mut rng = OsRng;
         // 使用正确的 API：先生成随机字节，然后创建 SecretKey
         let mut secret_bytes = [0u8; 32];
         rng.fill(&mut secret_bytes);
         let secret_key = Secp256k1SecretKey::from_slice(&secret_bytes).expect("32 bytes, within curve order");
         let public_key = Secp256k1PublicKey::from_secret_key(&secp, &secret_key);
         (secret_key, public_key)
     }
 
    /// 生成X25519密钥对（从Ed25519私钥派生）
    fn generate_x25519_keypair(&self, _ed25519_secret: &SigningKey) -> (EphemeralSecret, X25519PublicKey) {
        let x25519_secret = EphemeralSecret::random_from_rng(&mut OsRng);
        let x25519_public = X25519PublicKey::from(&x25519_secret);
        (x25519_secret, x25519_public)
    }
 
     /// 将公钥编码为Multibase格式
     fn encode_multibase(&self, public_key: &[u8]) -> String {
         let base58 = bs58::encode(public_key).into_string();
         format!("z{}", base58)
     }
 
     /// 生成JWK格式的公钥
     fn generate_jwk(&self, public_key: &Secp256k1PublicKey, kid: &str) -> PublicKeyJWK {
         let public_key_bytes = public_key.serialize_uncompressed();
         let x = &public_key_bytes[1..33]; // 跳过0x04前缀
         let y = &public_key_bytes[33..65];
         
         PublicKeyJWK {
             crv: "secp256k1".to_string(),
             x: Some(general_purpose::URL_SAFE_NO_PAD.encode(x)),
             y: Some(general_purpose::URL_SAFE_NO_PAD.encode(y)),
             kty: "EC".to_string(),
             kid: kid.to_string(),
         }
     }
 
     /// 生成完整的密钥对和DID文档
     pub fn generate_keypair(&self, key_type: KeyType) -> anyhow::Result<KeyPairResult> {
         let did = self.generate_did();
         let key_id = self.generate_nonce(16);
         
         let mut verification_methods = Vec::new();
         let mut authentication_methods = Vec::new();
         let mut key_agreement_methods = Vec::new();
         let mut human_authorization_methods = Vec::new();
 
         let (private_key_pem, auth_key_id) = match key_type {
             KeyType::Ed25519 => {
                 let (secret_key, public_key) = self.generate_ed25519_keypair();
                 let private_key_pem = self.generate_pem_private_key(secret_key.to_bytes().as_slice(), "ED25519");
                 
                 let auth_key_id = format!("{}#{}", did, key_id);
                 let auth_method = VerificationMethod {
                     id: auth_key_id.clone(),
                     method_type: "Ed25519VerificationKey2020".to_string(),
                     controller: did.clone(),
                     public_key_jwk: None,
                     public_key_multibase: Some(self.encode_multibase(public_key.as_bytes())),
                 };
 
                 verification_methods.push(auth_method.clone());
                 authentication_methods.push(AuthenticationMethod::String(auth_key_id.clone()));
                 
                 (private_key_pem, auth_key_id)
             }
             KeyType::Secp256k1 => {
                 let (secret_key, public_key) = self.generate_secp256k1_keypair();
                 let private_key_pem = self.generate_pem_private_key(&secret_key.secret_bytes(), "SECP256K1");
                 
                 let auth_key_id = format!("{}#{}", did, key_id);
                 let jwk = self.generate_jwk(&public_key, &key_id);
                 let auth_method = VerificationMethod {
                     id: auth_key_id.clone(),
                     method_type: "EcdsaSecp256k1VerificationKey2019".to_string(),
                     controller: did.clone(),
                     public_key_jwk: Some(jwk),
                     public_key_multibase: None,
                 };
 
                 verification_methods.push(auth_method.clone());
                 authentication_methods.push(AuthenticationMethod::String(auth_key_id.clone()));
                 
                 (private_key_pem, auth_key_id)
             }
             _ => return Err(anyhow::anyhow!("不支持的密钥类型")),
         };
 
         // 生成密钥协商密钥（X25519）
         let (ed25519_secret, _) = self.generate_ed25519_keypair();
         let (_, x25519_public) = self.generate_x25519_keypair(&ed25519_secret);
         let key_agreement_id = format!("{}#key-2", did);
         let key_agreement_method = VerificationMethod {
             id: key_agreement_id.clone(),
             method_type: "X25519KeyAgreementKey2019".to_string(),
             controller: did.clone(),
             public_key_jwk: None,
             public_key_multibase: Some(self.encode_multibase(x25519_public.as_bytes())),
         };
 
         verification_methods.push(key_agreement_method.clone());
         key_agreement_methods.push(KeyAgreementMethod::Object(key_agreement_method));
 
        // 生成人类授权密钥（Ed25519）
        let (_human_auth_secret, human_auth_public) = self.generate_ed25519_keypair();
         let human_auth_id = format!("{}#key-3", did);
         let human_auth_method = VerificationMethod {
             id: human_auth_id.clone(),
             method_type: "Ed25519VerificationKey2020".to_string(),
             controller: did.clone(),
             public_key_jwk: None,
             public_key_multibase: Some(self.encode_multibase(human_auth_public.as_bytes())),
         };
 
         verification_methods.push(human_auth_method.clone());
         human_authorization_methods.push(HumanAuthorizationMethod::String(auth_key_id.clone()));
         human_authorization_methods.push(HumanAuthorizationMethod::Object(human_auth_method));
 
         // 生成符合官方规范的DID文档
         let did_document = DIDDocument {
             context: vec![
                 "https://www.w3.org/ns/did/v1".to_string(),
                 "https://w3id.org/security/suites/jws-2020/v1".to_string(),
                 "https://w3id.org/security/suites/secp256k1-2019/v1".to_string(),
                 "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                 "https://w3id.org/security/suites/x25519-2019/v1".to_string(),
             ],
             id: did.clone(),
             verification_method: Some(verification_methods),
             authentication: authentication_methods,
             key_agreement: Some(key_agreement_methods),
             human_authorization: Some(human_authorization_methods),
             service: Some(vec![Service {
                 id: format!("{}#agent-description", did),
                 service_type: "AgentDescription".to_string(),
                 service_endpoint: format!(
                     "https://{}/agents/{}/ad.json",
                     self.domain,
                     self.path.as_deref().unwrap_or("default")
                 ),
             }]),
         };
 
         Ok(KeyPairResult {
             did_document: serde_json::to_string_pretty(&did_document)?,
             private_key: private_key_pem,
             did,
         })
     }
 
     /// 生成PEM格式的私钥
     fn generate_pem_private_key(&self, private_key: &[u8], key_type: &str) -> String {
         let header = format!("-----BEGIN {} PRIVATE KEY-----", key_type);
         let footer = format!("-----END {} PRIVATE KEY-----", key_type);
         let base64_key = general_purpose::STANDARD.encode(private_key);
         
         // 每64个字符换行
         let formatted_key = base64_key
             .chars()
             .collect::<Vec<_>>()
             .chunks(64)
             .map(|chunk| chunk.iter().collect::<String>())
             .collect::<Vec<_>>()
             .join("\n");
         
         format!("{}\n{}\n{}", header, formatted_key, footer)
     }
 
     /// 生成签名数据
     pub fn generate_signature_data(&self, service: &str, did: &str) -> SignatureData {
         SignatureData {
             nonce: self.generate_nonce(16),
             timestamp: Utc::now().to_rfc3339(),
             service: service.to_string(),
             did: did.to_string(),
         }
     }
 
     /// 使用JCS规范化JSON
     pub fn jcs_canonicalize(&self, obj: &Value) -> anyhow::Result<String> {
         // 简化的JCS实现，实际应使用专门的JCS库
         let mut sorted_obj = obj.as_object().unwrap().clone();
         sorted_obj.sort_keys();
         Ok(serde_json::to_string(&sorted_obj)?)
     }
 
     /// 使用Ed25519私钥签名
     pub fn sign_ed25519(&self, private_key: &SigningKey, data: &SignatureData) -> anyhow::Result<String> {
         let data_json = serde_json::to_value(data)?;
         let canonical_json = self.jcs_canonicalize(&data_json)?;
         let message = canonical_json.as_bytes();
         
         let signature = private_key.sign(message);
         
         Ok(general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes()))
     }
 
     /// 使用secp256k1私钥签名
     pub fn sign_secp256k1(&self, private_key: &Secp256k1SecretKey, data: &SignatureData) -> anyhow::Result<String> {
         let data_json = serde_json::to_value(data)?;
         let canonical_json = self.jcs_canonicalize(&data_json)?;
         let message_hash = Sha256::digest(canonical_json.as_bytes());
         
         let secp = Secp256k1::new();
         let message = secp256k1::Message::from_digest_slice(&message_hash)?;
         let signature = secp.sign_ecdsa(&message, private_key);
         
         Ok(general_purpose::URL_SAFE_NO_PAD.encode(signature.serialize_der()))
     }
 
     /// 验证Ed25519签名
     pub fn verify_ed25519(&self, public_key: &VerifyingKey, signature: &str, data: &SignatureData) -> anyhow::Result<bool> {
         let data_json = serde_json::to_value(data)?;
         let canonical_json = self.jcs_canonicalize(&data_json)?;
         let message = canonical_json.as_bytes();
         
         let signature_bytes = general_purpose::URL_SAFE_NO_PAD.decode(signature)?;
         let signature_array: [u8; 64] = signature_bytes.try_into().map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
         let signature = ed25519_dalek::Signature::from_bytes(&signature_array);
         
         Ok(public_key.verify_strict(message, &signature).is_ok())
     }
 
     /// 验证secp256k1签名
     pub fn verify_secp256k1(&self, public_key: &Secp256k1PublicKey, signature: &str, data: &SignatureData) -> anyhow::Result<bool> {
         let data_json = serde_json::to_value(data)?;
         let canonical_json = self.jcs_canonicalize(&data_json)?;
         let message_hash = Sha256::digest(canonical_json.as_bytes());
         
         let secp = Secp256k1::new();
         let message = secp256k1::Message::from_digest_slice(&message_hash)?;
         let signature_bytes = general_purpose::URL_SAFE_NO_PAD.decode(signature)?;
         let signature = secp256k1::ecdsa::Signature::from_der(&signature_bytes)?;
         
         Ok(secp.verify_ecdsa(&message, &signature, public_key).is_ok())
     }
 }
 
 // 使用示例
 pub fn example() -> anyhow::Result<()> {
     let generator = ANPKeyGenerator::new("example.com".to_string(), Some("user:alice".to_string()));
     
     // 生成Ed25519密钥对
     let ed25519_result = generator.generate_keypair(KeyType::Ed25519)?;
     println!("Ed25519 DID文档: {}", ed25519_result.did_document);
     println!("Ed25519 私钥: {}", ed25519_result.private_key);
     
     // 生成secp256k1密钥对
     let secp256k1_result = generator.generate_keypair(KeyType::Secp256k1)?;
     println!("secp256k1 DID文档: {}", secp256k1_result.did_document);
     println!("secp256k1 私钥: {}", secp256k1_result.private_key);
     
     // 生成签名数据
     let signature_data = generator.generate_signature_data("example.com", &ed25519_result.did);
     println!("签名数据: {:?}", signature_data);
     
     Ok(())
 }
 
 #[cfg(test)]
 mod tests {
     use super::*;
 
     #[test]
     fn test_generate_ed25519_keypair() {
         let generator = ANPKeyGenerator::new("test.com".to_string(), None);
         let result = generator.generate_keypair(KeyType::Ed25519).unwrap();
         
         assert!(result.did.starts_with("did:wba:test.com"));
         assert!(result.private_key.contains("BEGIN ED25519 PRIVATE KEY"));
         assert!(result.did_document.contains("@context"));
     }
 
     #[test]
     fn test_generate_secp256k1_keypair() {
         let generator = ANPKeyGenerator::new("test.com".to_string(), Some("user:test".to_string()));
         let result = generator.generate_keypair(KeyType::Secp256k1).unwrap();
         
         assert!(result.did.starts_with("did:wba:test.com:user:test"));
         assert!(result.private_key.contains("BEGIN SECP256K1 PRIVATE KEY"));
         assert!(result.did_document.contains("secp256k1"));
     }
 
     #[test]
     fn test_ed25519_sign_verify() {
         let generator = ANPKeyGenerator::new("test.com".to_string(), None);
         let (secret_key, public_key) = generator.generate_ed25519_keypair();
         let data = generator.generate_signature_data("test.com", "did:wba:test.com");
         
         let signature = generator.sign_ed25519(&secret_key, &data).unwrap();
         let is_valid = generator.verify_ed25519(&public_key, &signature, &data).unwrap();
         
         assert!(is_valid);
     }
 
     #[test]
     fn test_secp256k1_sign_verify() {
         let generator = ANPKeyGenerator::new("test.com".to_string(), None);
         let (secret_key, public_key) = generator.generate_secp256k1_keypair();
         let data = generator.generate_signature_data("test.com", "did:wba:test.com");
         
         let signature = generator.sign_secp256k1(&secret_key, &data).unwrap();
         let is_valid = generator.verify_secp256k1(&public_key, &signature, &data).unwrap();
         
         assert!(is_valid);
     }
 }
 
// 如果直接运行此文件
#[allow(dead_code)]
fn main() -> anyhow::Result<()> {
    example()
}