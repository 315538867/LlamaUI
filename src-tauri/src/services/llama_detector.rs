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
    let mut installs = Vec::new();

    // Check PATH first
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(separator) {
            if let Some(install) = check_directory(dir) {
                if !installs.iter().any(|i: &LlamaInstall| i.path == install.path) {
                    installs.push(install);
                }
            }
        }
    }

    // Check common Windows locations
    if cfg!(windows) {
        let common_paths = [
            r"C:\llama.cpp",
            r"C:\Program Files\llama.cpp",
            r"C:\Program Files (x86)\llama.cpp",
        ];
        // Also check user home
        if let Ok(home) = std::env::var("USERPROFILE") {
            let home_paths = [
                format!("{}\\.llama.cpp", home),
                format!("{}\\llama.cpp", home),
                format!("{}\\AppData\\Local\\llama.cpp", home),
            ];
            for p in home_paths {
                if let Some(install) = check_directory(&p) {
                    if !installs.iter().any(|i| i.path == install.path) {
                        installs.push(install);
                    }
                }
            }
        }
        for p in common_paths {
            if let Some(install) = check_directory(p) {
                if !installs.iter().any(|i| i.path == install.path) {
                    installs.push(install);
                }
            }
        }
    }

    // macOS / Linux common paths
    if cfg!(not(windows)) {
        let common_paths = [
            "/usr/local/bin",
            "/opt/homebrew/bin",
        ];
        for p in common_paths {
            if let Some(install) = check_directory(p) {
                if !installs.iter().any(|i| i.path == install.path) {
                    installs.push(install);
                }
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
        path: dir.to_string(),
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
