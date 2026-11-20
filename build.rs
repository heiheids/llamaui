//! 构建脚本：注入版本信息，并把图标 / 版本资源编译进可执行文件
//! Build script: injects version metadata and compiles the icon resource.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    println!("cargo:rustc-env=APP_VERSION={}", version);
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=locales");

    embed_icon();
}

/// 用 windres 把 assets/app.rc（内含 icon.ico）编译成目标文件并链接进 exe，
/// 这样资源管理器里显示的就是项目自带图标，运行时也能用 LoadImageW 取出。
fn embed_icon() {
    // 仅 Windows 需要；其它平台直接跳过
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    let rc = PathBuf::from("assets/app.rc");
    let ico = PathBuf::from("assets/icon.ico");
    if !rc.is_file() || !ico.is_file() {
        println!("cargo:warning=assets/app.rc 或 assets/icon.ico 缺失，跳过图标资源");
        return;
    }

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let tool = match arch.as_str() {
        "x86_64" => "x86_64-w64-mingw32-windres",
        "x86" => "i686-w64-mingw32-windres",
        _ => "windres",
    };

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| ".".into()));
    let obj = out_dir.join("app_icon.o");

    let mut candidates: Vec<String> = vec![tool.to_string(), "windres".to_string()];
    // 工程内自带的工具链
    if let Ok(root) = std::env::var("CARGO_MANIFEST_DIR") {
        let bin = PathBuf::from(root).join(".toolchain/w64devkit/bin");
        candidates.insert(0, bin.join(format!("{}.exe", tool)).to_string_lossy().to_string());
        candidates.insert(1, bin.join("windres.exe").to_string_lossy().to_string());
    }

    for cmd in candidates {
        let status = Command::new(&cmd)
            .arg(&rc)
            .arg("-O")
            .arg("coff")
            .arg("-o")
            .arg(&obj)
            .arg("--include-dir")
            .arg("assets")
            .arg("--codepage")
            .arg("65001")
            .status();
        if let Ok(st) = status {
            if st.success() {
                println!("cargo:rustc-link-arg={}", obj.display());
                println!("cargo:rerun-if-changed=assets/icon.ico");
                println!("cargo:rerun-if-changed=assets/app.rc");
                return;
            }
        }
    }
    println!("cargo:warning=未找到 windres，图标资源未嵌入（程序仍可运行）");
}
