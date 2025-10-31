// DIAP Rust SDK - Kubo自动安装器
// 自动下载并安装Kubo (go-ipfs)二进制文件，实现零配置部署

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File, Permissions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;
use log;

/// Kubo安装器
/// 负责自动下载和安装Kubo二进制文件
pub struct KuboInstaller {
    install_dir: PathBuf,
    version: String,
}

impl KuboInstaller {
    /// 创建新的Kubo安装器
    pub fn new() -> Self {
        // 使用用户主目录下的固定位置
        let install_dir = dirs::home_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join(".diap")
            .join("kubo");
        
        Self {
            install_dir,
            version: "v0.32.1".to_string(), // Kubo最新稳定版本
        }
    }
    
    /// 确保Kubo已安装，如果未安装则自动下载安装
    pub async fn ensure_kubo_installed(&self) -> Result<PathBuf> {
        let ipfs_path = self.get_kubo_path();
        
        // 检查Kubo是否已存在
        if ipfs_path.exists() {
            log::info!("✓ 检测到已安装的Kubo: {:?}", ipfs_path);
            
            // 验证可执行文件是否有效
            if self.verify_kubo(&ipfs_path)? {
                log::info!("✓ Kubo验证成功");
                return Ok(ipfs_path);
            } else {
                log::warn!("Kubo文件损坏，重新下载...");
            }
        }
        
        // 下载并安装
        log::info!("📥 开始下载Kubo ({})...", self.version);
        self.download_and_install().await?;
        
        // 验证安装
        if !self.verify_kubo(&ipfs_path)? {
            anyhow::bail!("Kubo安装后验证失败");
        }
        
        log::info!("✅ Kubo安装完成: {:?}", ipfs_path);
        Ok(ipfs_path)
    }
    
    /// 获取Kubo可执行文件路径
    pub fn get_kubo_path(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        return self.install_dir.join("ipfs.exe");
        
        #[cfg(not(target_os = "windows"))]
        return self.install_dir.join("ipfs");
    }
    
    /// 下载并安装Kubo
    async fn download_and_install(&self) -> Result<()> {
        // 创建安装目录
        fs::create_dir_all(&self.install_dir)
            .context("无法创建Kubo安装目录")?;
        
        // 构建下载URL
        let download_url = self.build_download_url()?;
        log::info!("  下载URL: {}", download_url);
        
        // 下载文件
        let temp_file = self.download_file(&download_url).await?;
        
        // 解压文件
        self.extract_kubo(&temp_file)?;
        
        // 设置可执行权限（Unix系统）
        #[cfg(not(target_os = "windows"))]
        self.set_executable_permissions()?;
        
        Ok(())
    }
    
    /// 构建下载URL
    fn build_download_url(&self) -> Result<String> {
        let (os, arch) = self.get_platform_info()?;
        
        let filename = format!("kubo_{}_{}_{}.tar.gz", self.version, os, arch);
        let url = format!("https://dist.ipfs.tech/kubo/{}/{}", self.version, filename);
        
        Ok(url)
    }
    
    /// 获取平台信息（操作系统和架构）
    fn get_platform_info(&self) -> Result<(String, String)> {
        let os = match std::env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "darwin",
            other => anyhow::bail!("不支持的操作系统: {}", other),
        };
        
        let arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" | "arm64" => "arm64",
            other => anyhow::bail!("不支持的架构: {}", other),
        };
        
        Ok((os.to_string(), arch.to_string()))
    }
    
    /// 下载文件到临时目录
    async fn download_file(&self, url: &str) -> Result<PathBuf> {
        use tokio::io::AsyncWriteExt;
        
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .context("下载请求失败")?;
        
        if !response.status().is_success() {
            anyhow::bail!("下载失败: HTTP {}", response.status());
        }
        
        let total_size = response.content_length();
        let mut downloaded = 0u64;
        
        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("kubo_{}.tar.gz", uuid::Uuid::new_v4()));
        let mut file = tokio::fs::File::create(&temp_file)
            .await
            .context("无法创建临时文件")?;
        
        // 下载并写入文件
        let mut stream = response.bytes_stream();
        use futures::StreamExt;
        
        while let Some(item) = stream.next().await {
            let chunk = item.context("下载流错误")?;
            file.write_all(&chunk)
                .await
                .context("写入文件失败")?;
            
            downloaded += chunk.len() as u64;
            
            if let Some(total) = total_size {
                let percent = (downloaded as f64 / total as f64) * 100.0;
                if downloaded % (total / 100 + 1) == 0 {
                    log::info!("  下载进度: {:.1}% ({}/{} bytes)", percent, downloaded, total);
                }
            }
        }
        
        log::info!("✓ 下载完成: {} bytes", downloaded);
        Ok(temp_file)
    }
    
    /// 解压Kubo归档文件
    fn extract_kubo(&self, archive_path: &Path) -> Result<()> {
        log::info!("📦 解压Kubo文件...");
        
        let file = File::open(archive_path)
            .context("无法打开归档文件")?;
        let decoder = GzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);
        
        // 解压所有文件
        for entry in archive.entries().context("读取归档失败")? {
            let mut entry = entry.context("读取归档条目失败")?;
            let path = entry.path().context("获取归档路径失败")?;
            
            // 只提取kubo目录下的文件
            let path_str = path.to_string_lossy();
            if !path_str.contains("kubo/") {
                continue;
            }
            
            // 获取文件名
            let filename = path.file_name()
                .ok_or_else(|| anyhow::anyhow!("无法获取文件名"))?;
            
            // 跳过不是ipfs文件的项目
            #[cfg(target_os = "windows")]
            if filename.to_string_lossy() != "ipfs.exe" {
                continue;
            }
            
            #[cfg(not(target_os = "windows"))]
            if filename.to_string_lossy() != "ipfs" {
                continue;
            }
            
            // 解压到安装目录
            let out_path = self.install_dir.join(filename);
            entry.unpack(&out_path)
                .context(format!("解压到 {:?} 失败", out_path))?;
            
            log::info!("✓ 解压完成: {:?}", out_path);
        }
        
        Ok(())
    }
    
    /// 设置可执行权限（Unix系统）
    #[cfg(not(target_os = "windows"))]
    fn set_executable_permissions(&self) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        
        let ipfs_path = self.get_kubo_path();
        let mut perms = fs::metadata(&ipfs_path)
            .context("无法获取文件元数据")?
            .permissions();
        
        // 设置用户可执行权限
        perms.set_mode(0o755);
        fs::set_permissions(&ipfs_path, perms)
            .context("无法设置可执行权限")?;
        
        log::info!("✓ 设置可执行权限完成");
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn set_executable_permissions(&self) -> Result<()> {
        // Windows不需要设置可执行权限
        Ok(())
    }
    
    /// 验证Kubo可执行文件
    fn verify_kubo(&self, kubo_path: &Path) -> Result<bool> {
        if !kubo_path.exists() {
            return Ok(false);
        }
        
        // 尝试执行 --version 命令
        let output = Command::new(kubo_path)
            .arg("--version")
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }
    
    /// 获取Kubo版本
    pub fn get_version(&self) -> &str {
        &self.version
    }
    
    /// 获取安装目录
    pub fn get_install_dir(&self) -> &Path {
        &self.install_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // 跳过下载测试，避免CI/CD耗时
    async fn test_kubo_installer() {
        let installer = KuboInstaller::new();
        let path = installer.ensure_kubo_installed().await;
        
        match path {
            Ok(p) => println!("Kubo安装路径: {:?}", p),
            Err(e) => panic!("安装失败: {}", e),
        }
    }
    
    #[test]
    fn test_platform_info() {
        let installer = KuboInstaller::new();
        let (os, arch) = installer.get_platform_info().unwrap();
        
        println!("操作系统: {}", os);
        println!("架构: {}", arch);
        
        assert!(!os.is_empty());
        assert!(!arch.is_empty());
    }
    
    #[test]
    fn test_download_url() {
        let installer = KuboInstaller::new();
        let url = installer.build_download_url().unwrap();
        
        println!("下载URL: {}", url);
        assert!(url.starts_with("https://dist.ipfs.tech/kubo/"));
    }
}
