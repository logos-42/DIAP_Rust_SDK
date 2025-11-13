// DIAP Rust SDK - 构建脚本
// 处理Noir电路的预编译和嵌入

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=noir_circuits/");
    println!("cargo:rerun-if-changed=build.rs");

    // 检查是否启用了嵌入Noir功能
    if cfg!(feature = "embedded-noir") {
        println!("cargo:rustc-cfg=feature=\"embedded-noir\"");

        // 尝试预编译Noir电路
        if let Err(e) = precompile_noir_circuits() {
            println!("cargo:warning=Failed to precompile Noir circuits: {}", e);
            println!("cargo:warning=SDK will use fallback ZKP implementation");
        } else {
            println!("cargo:rustc-cfg=feature=\"noir-precompiled\"");
        }
    }

    // 检查IPFS可用性
    if check_ipfs_available() {
        println!("cargo:rustc-cfg=feature=\"ipfs-available\"");
    }

    // 检查Noir可用性
    if check_noir_available() {
        println!("cargo:rustc-cfg=feature=\"noir-available\"");
    }
}

/// 预编译Noir电路
fn precompile_noir_circuits() -> Result<(), Box<dyn std::error::Error>> {
    let circuit_dir = "noir_circuits";

    if !Path::new(circuit_dir).exists() {
        return Err("Noir circuits directory not found".into());
    }

    // 检查nargo是否可用
    let nargo_available = Command::new("nargo")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !nargo_available {
        println!("cargo:warning=nargo not available, skipping precompilation");
        return Ok(());
    }

    // 编译电路
    let compile_result = Command::new("nargo")
        .arg("compile")
        .current_dir(circuit_dir)
        .output()?;

    if !compile_result.status.success() {
        let error = String::from_utf8_lossy(&compile_result.stderr);
        return Err(format!("Noir compilation failed: {}", error).into());
    }

    // 确保目标目录存在
    let target_dir = format!("{}/target", circuit_dir);
    if !Path::new(&target_dir).exists() {
        return Err("Noir compilation target directory not found".into());
    }

    // 检查关键文件是否存在
    let acir_file = format!("{}/noir_circuits.json", target_dir);
    let witness_file = format!("{}/noir_circuits.gz", target_dir);

    if !Path::new(&acir_file).exists() {
        return Err("ACIR file not generated".into());
    }

    if !Path::new(&witness_file).exists() {
        return Err("Witness file not generated".into());
    }

    println!("cargo:warning=Noir circuits precompiled successfully");
    Ok(())
}

/// 检查IPFS是否可用
fn check_ipfs_available() -> bool {
    Command::new("ipfs")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 检查Noir是否可用
fn check_noir_available() -> bool {
    Command::new("nargo")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
