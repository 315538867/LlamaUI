use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaInstall {
    pub path: String,
    pub version: Option<String>,
    pub has_server: bool,
    pub has_cli: bool,
}

/// Search common locations for llama.cpp binaries
pub fn detect_installations() -> Vec<LlamaInstall> {
    let mut installs: Vec<LlamaInstall> = Vec::new();

    let mut try_add = |dir: &str| {
        let dir = dir.trim().trim_matches('"');
        if dir.is_empty() {
            return;
        }
        if let Some(install) = check_directory(dir) {
            if !installs.iter().any(|i| i.path == install.path) {
                installs.push(install);
            }
        }
    };

    // Check PATH first
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(separator) {
            try_add(dir);
        }
    }

    #[cfg(windows)]
    {
        // Common fixed locations
        let fixed: &[&str] = &[
            r"C:\llama.cpp",
            r"C:\llama.cpp\build\bin\Release",
            r"C:\llama.cpp\build\Release",
            r"C:\llama.cpp\bin",
            r"C:\Program Files\llama.cpp",
            r"C:\Program Files\llama.cpp\bin",
            r"C:\ai\llama.cpp",
            r"C:\ai\llama.cpp\bin",
            r"D:\llama.cpp",
            r"D:\llama.cpp\bin",
            r"D:\ai\llama.cpp",
        ];
        for p in fixed {
            try_add(p);
        }

        // User-relative locations
        for var in &["USERPROFILE", "HOMEPATH"] {
            if let Ok(home) = std::env::var(var) {
                let candidates = [
                    format!(r"{}\llama.cpp", home),
                    format!(r"{}\llama.cpp\build\bin\Release", home),
                    format!(r"{}\llama.cpp\bin", home),
                    format!(r"{}\AppData\Local\llama.cpp", home),
                    format!(r"{}\AppData\Local\llama.cpp\bin", home),
                    // Common pattern: extracted GitHub release zip
                    format!(r"{}\Downloads\llama.cpp", home),
                    format!(r"{}\Downloads\llama.cpp\bin", home),
                ];
                for p in &candidates {
                    try_add(p);
                }
            }
        }

        // Also scan Downloads for release zips (llama-bNNNN-bin-win-*)
        if let Ok(home) = std::env::var("USERPROFILE") {
            let downloads = std::path::Path::new(&home).join("Downloads");
            if let Ok(entries) = std::fs::read_dir(&downloads) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.starts_with("llama") && entry.path().is_dir() {
                        try_add(&entry.path().to_string_lossy());
                        try_add(&entry.path().join("bin").to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let fixed: &[&str] = &[
            "/usr/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/local/bin",
        ];
        for p in fixed {
            try_add(p);
        }

        // User home relative
        if let Ok(home) = std::env::var("HOME") {
            let candidates = [
                format!("{}/llama.cpp/build/bin", home),
                format!("{}/llama.cpp/build/Release", home),
                format!("{}/llama.cpp/bin", home),
                format!("{}/llama.cpp", home),
                format!("{}/.local/bin", home),
                format!("{}/Downloads/llama.cpp/build/bin", home),
                format!("{}/Downloads/llama.cpp", home),
            ];
            for p in &candidates {
                try_add(p);
            }
        }
    }

    installs
}

fn check_directory(dir: &str) -> Option<LlamaInstall> {
    let dir_path = Path::new(dir);
    if !dir_path.exists() {
        return None;
    }

    let server_name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };
    let cli_name = if cfg!(windows) { "llama-cli.exe" } else { "llama-cli" };

    let has_server = dir_path.join(server_name).exists();
    let has_cli = dir_path.join(cli_name).exists();

    if !has_server && !has_cli {
        return None;
    }

    Some(LlamaInstall {
        path: dir_path.to_string_lossy().into_owned(),
        version: None,
        has_server,
        has_cli,
    })
}

pub fn validate_path(path: &str) -> Result<LlamaInstall, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("路径不存在".into());
    }

    let server_name = if cfg!(windows) { "llama-server.exe" } else { "llama-server" };
    let cli_name = if cfg!(windows) { "llama-cli.exe" } else { "llama-cli" };

    let has_server = p.join(server_name).exists();
    let has_cli = p.join(cli_name).exists();

    if !has_server && !has_cli {
        // Maybe the path points directly to a binary
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.contains("llama-server") || name.contains("llama-cli") {
                let parent = p.parent().map(|pp| pp.to_string_lossy().to_string()).unwrap_or_default();
                return Ok(LlamaInstall {
                    path: parent,
                    version: None,
                    has_server: name.contains("llama-server"),
                    has_cli: name.contains("llama-cli"),
                });
            }
        }
        return Err("目录中未找到 llama-server 或 llama-cli".into());
    }

    Ok(LlamaInstall {
        path: path.to_string(),
        version: None,
        has_server,
        has_cli,
    })
}

pub fn get_binary_path(llama_dir: &str, binary: &str) -> PathBuf {
    let ext = if cfg!(windows) { ".exe" } else { "" };
    Path::new(llama_dir).join(format!("{}{}", binary, ext))
}
