use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_display: String,
    pub quantization: Option<String>,
    pub modified: Option<u64>,
}

/// Scan a directory for .gguf files
pub fn scan_directory(dir: &str) -> Result<Vec<ModelInfo>, String> {
    let path = Path::new(dir);
    if !path.exists() {
        return Err(format!("目录不存在: {}", dir));
    }
    if !path.is_dir() {
        return Err(format!("不是目录: {}", dir));
    }

    let mut models = Vec::new();
    scan_recursive(path, &mut models, 0);
    models.sort_by_key(|m| m.name.to_lowercase());
    Ok(models)
}

const MAX_DEPTH: u32 = 6;

fn scan_recursive(dir: &Path, models: &mut Vec<ModelInfo>, depth: u32) {
    if depth >= MAX_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_recursive(&path, models, depth + 1);
        } else if let Some(ext) = path.extension() {
            if ext.to_str().unwrap_or("").to_lowercase() == "gguf" {
                if let Some(info) = parse_model_file(&path) {
                    models.push(info);
                }
            }
        }
    }
}

pub fn parse_model_file(path: &Path) -> Option<ModelInfo> {
    let metadata = std::fs::metadata(path).ok()?;
    let size = metadata.len();
    let name = path.file_stem()?.to_str()?.to_string();
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Try to infer quantization from filename
    let quantization = infer_quantization(&name);

    Some(ModelInfo {
        name,
        path: path.to_str()?.to_string(),
        size_bytes: size,
        size_display: format_size(size),
        quantization,
        modified,
    })
}

fn infer_quantization(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    let quant_patterns = [
        "Q2_K", "Q3_K_S", "Q3_K_M", "Q3_K_L",
        "Q4_0", "Q4_1", "Q4_K_S", "Q4_K_M",
        "Q5_0", "Q5_1", "Q5_K_S", "Q5_K_M",
        "Q6_K", "Q8_0", "F16", "F32",
        "IQ1_S", "IQ1_M", "IQ2_XXS", "IQ2_XS", "IQ2_S", "IQ2_M",
        "IQ3_XXS", "IQ3_XS", "IQ3_S", "IQ3_M",
        "IQ4_NL", "IQ4_XS",
    ];

    for pattern in quant_patterns {
        if upper.contains(pattern) {
            return Some(pattern.to_string());
        }
    }
    None
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
