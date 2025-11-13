// DIAP Rust SDK - libp2p身份管理模块
// Decentralized Intelligent Agent Protocol
// 管理libp2p密钥对和PeerID，与IPNS密钥分离

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use libp2p::identity::Keypair;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// libp2p身份信息
#[derive(Clone)]
pub struct LibP2PIdentity {
    /// libp2p密钥对
    keypair: Keypair,

    /// PeerID（从密钥对派生）
    peer_id: PeerId,
}

/// libp2p密钥文件格式
#[derive(Debug, Serialize, Deserialize)]
struct LibP2PKeyFile {
    /// 密钥类型
    key_type: String,

    /// Protobuf编码的密钥对
    keypair_protobuf: String,

    /// PeerID
    peer_id: String,

    /// 创建时间
    created_at: String,

    /// 版本
    version: String,
}

impl LibP2PIdentity {
    /// 生成新的libp2p身份
    pub fn generate() -> Result<Self> {
        // 生成Ed25519密钥对（推荐，与IPFS兼容）
        let keypair = Keypair::generate_ed25519();

        // 从密钥对自动派生PeerID
        let peer_id = PeerId::from(keypair.public());

        log::info!("生成新的libp2p身份");
        log::info!("  PeerID: {}", peer_id);

        Ok(Self { keypair, peer_id })
    }

    /// 从protobuf编码的bytes加载
    pub fn from_protobuf_encoding(bytes: &[u8]) -> Result<Self> {
        let keypair = Keypair::from_protobuf_encoding(bytes).context("无法从protobuf解码密钥对")?;

        let peer_id = PeerId::from(keypair.public());

        Ok(Self { keypair, peer_id })
    }

    /// 从文件加载
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取libp2p密钥文件: {:?}", path))?;

        let key_file: LibP2PKeyFile = serde_json::from_str(&content)
            .with_context(|| format!("无法解析libp2p密钥文件: {:?}", path))?;

        // 解码protobuf
        let keypair_bytes = general_purpose::STANDARD
            .decode(&key_file.keypair_protobuf)
            .context("无法解码密钥对")?;

        Self::from_protobuf_encoding(&keypair_bytes)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建目录: {:?}", parent))?;
        }

        // 编码为protobuf
        let keypair_bytes = self
            .keypair
            .to_protobuf_encoding()
            .context("无法编码密钥对")?;

        // 检测密钥类型
        let key_type = format!("{:?}", self.keypair.public().key_type());

        let key_file = LibP2PKeyFile {
            key_type,
            keypair_protobuf: general_purpose::STANDARD.encode(&keypair_bytes),
            peer_id: self.peer_id.to_base58(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
        };

        let content = serde_json::to_string_pretty(&key_file).context("无法序列化密钥")?;

        std::fs::write(path, content).with_context(|| format!("无法写入密钥文件: {:?}", path))?;

        // 设置文件权限为600（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        log::info!("libp2p密钥已保存到: {:?}", path);
        Ok(())
    }

    /// 获取PeerID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// 获取PeerID的Base58字符串
    pub fn peer_id_string(&self) -> String {
        self.peer_id.to_base58()
    }

    /// 获取密钥对引用
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// 获取公钥的multibase编码
    pub fn public_key_multibase(&self) -> String {
        let public_key_bytes = self.keypair.public().encode_protobuf();
        format!("z{}", bs58::encode(&public_key_bytes).into_string())
    }
}

impl std::fmt::Debug for LibP2PIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibP2PIdentity")
            .field("peer_id", &self.peer_id)
            .finish()
    }
}

/// libp2p身份管理器
pub struct LibP2PIdentityManager {
    #[allow(dead_code)]
    config_dir: PathBuf,
}

impl LibP2PIdentityManager {
    /// 创建新的身份管理器
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// 加载或生成libp2p身份
    pub fn load_or_generate(&self, key_path: &PathBuf) -> Result<LibP2PIdentity> {
        if key_path.exists() {
            log::info!("从文件加载libp2p身份: {:?}", key_path);
            LibP2PIdentity::from_file(key_path)
        } else {
            log::info!("生成新的libp2p身份");
            let identity = LibP2PIdentity::generate()?;
            identity.save_to_file(key_path)?;
            Ok(identity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_identity() {
        let identity = LibP2PIdentity::generate().unwrap();
        let peer_id_str = identity.peer_id_string();

        // Ed25519 PeerID通常以12D3开头
        assert!(!peer_id_str.is_empty());
        println!("生成的PeerID: {}", peer_id_str);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("libp2p.key");

        let identity1 = LibP2PIdentity::generate().unwrap();
        let peer_id1 = identity1.peer_id().clone();

        identity1.save_to_file(&key_path).unwrap();

        let identity2 = LibP2PIdentity::from_file(&key_path).unwrap();
        let peer_id2 = identity2.peer_id().clone();

        assert_eq!(peer_id1, peer_id2);
    }

    #[test]
    fn test_public_key_multibase() {
        let identity = LibP2PIdentity::generate().unwrap();
        let multibase = identity.public_key_multibase();

        // Multibase格式应该以'z'开头
        assert!(multibase.starts_with('z'));
        println!("公钥multibase: {}", multibase);
    }
}
