//! 构建脚本：注入版本信息，并把图标 / 版本资源编译进可执行文件
//! Build script: injects version metadata and compiles the icon resource.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    println!("cargo:rustc-env=APP_VERSION={}", version);
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=Cargo.toml");

    embed_icon(&version);
}

/// 把 "1.1.0" 转成 VERSIONINFO 需要的两个形式：
/// 数值 1,1,0,0 和字符串 "1.1.0.0"
fn version_forms(version: &str) -> (String, String) {
    let mut parts: Vec<String> = version
        .split('.')
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .collect();
    while parts.len() < 4 {
        parts.push("0".to_string());
    }
    parts.truncate(4);
    for p in parts.iter_mut() {
        if p.is_empty() {
            *p = "0".to_string();
        }
    }
    let numeric = parts.join(",");
    let dotted = parts.join(".");
    (numeric, dotted)
}

/// 用 windres 把 assets/app.rc（内含 icon.ico）编译成目标文件并链接进 exe，
/// 这样资源管理器里显示的就是项目自带图标，运行时也能用 LoadImageW 取出。
fn embed_icon(version: &str) {
    // 仅 Windows 需要；其它平台直接跳过
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }
    let rc_src = PathBuf::from("assets/app.rc");
    let ico = PathBuf::from("assets/icon.ico");
    if !rc_src.is_file() || !ico.is_file() {
        println!("cargo:warning=assets/app.rc 或 assets/icon.ico 缺失，跳过图标资源");
        return;
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| ".".into()));
    let obj = out_dir.join("app_icon.o");

    // 版本号从 Cargo.toml 注入：复制一份 rc 到构建目录并替换占位符，
    // 这样发版只需要改 Cargo.toml，不用再手改 app.rc
    let (numeric, dotted) = version_forms(version);
    let rc = match std::fs::read_to_string(&rc_src) {
        Ok(src) => {
            let patched = src
                .replace("@VERSION_NUMERIC@", &numeric)
                .replace("@VERSION_DOTTED@", &dotted);
            let dst = out_dir.join("app_versioned.rc");
            match std::fs::write(&dst, patched) {
                Ok(_) => dst,
                Err(_) => rc_src.clone(),
            }
        }
        Err(_) => rc_src.clone(),
    };
    // icon.ico 用相对 "icon.ico" 引用，所以要显式给出资源目录
    let include_dir = if rc == rc_src {
        PathBuf::from("assets")
    } else {
        PathBuf::from("assets")
    };

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let tool = match arch.as_str() {
        "x86_64" => "x86_64-w64-mingw32-windres",
        "x86" => "i686-w64-mingw32-windres",
        _ => "windres",
    };

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
            .arg(&include_dir)
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
