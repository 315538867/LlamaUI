use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::AppError;
use crate::services::llama_detector::get_binary_path;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BenchParams {
    pub threads: Option<u32>,
    pub prompt_tokens: u32,
    pub gen_tokens: u32,
}

#[derive(Clone, Serialize)]
struct BenchmarkLineEvent {
    instance: String,
    line: String,
}

#[derive(Clone, Serialize)]
struct BenchmarkDoneEvent {
    instance: String,
    exit_code: i32,
}

// ── Command ───────────────────────────────────────────────────────────────────

/// Run llama-bench for the given model instance.
/// Streams each output line as `llama://bench/line` and emits `llama://bench/done` on exit.
#[tauri::command]
pub async fn run_benchmark(
    app: AppHandle,
    instance_name: String,
    model_path: String,
    llama_dir: String,
    params: BenchParams,
) -> Result<(), AppError> {
    let bin = get_binary_path(&llama_dir, "llama-bench");
    if !bin.exists() {
        return Err(AppError::ProcessFailed {
            reason: format!("找不到 llama-bench: {}", bin.display()),
        });
    }

    let mut args: Vec<String> = vec![
        "-m".into(), model_path,
        "-p".into(), params.prompt_tokens.to_string(),
        "-n".into(), params.gen_tokens.to_string(),
    ];
    if let Some(t) = params.threads {
        args.extend_from_slice(&["-t".into(), t.to_string()]);
    }

    let mut child = Command::new(&bin)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::ProcessFailed { reason: e.to_string() })?;

    // Spawn stdout reader
    let stdout = child.stdout.take().expect("stdout handle");
    let app1 = app.clone();
    let name1 = instance_name.clone();
    let h1 = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            app1.emit("llama://bench/line", BenchmarkLineEvent {
                instance: name1.clone(),
                line,
            }).ok();
        }
    });

    // Spawn stderr reader
    let stderr = child.stderr.take().expect("stderr handle");
    let app2 = app.clone();
    let name2 = instance_name.clone();
    let h2 = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            app2.emit("llama://bench/line", BenchmarkLineEvent {
                instance: name2.clone(),
                line,
            }).ok();
        }
    });

    // Wait for child to exit, then drain readers
    let status = child.wait().await
        .map_err(|e| AppError::ProcessFailed { reason: e.to_string() })?;
    let _ = tokio::join!(h1, h2); // ensure all lines are emitted before done

    let exit_code = status.code().unwrap_or(-1);
    app.emit("llama://bench/done", BenchmarkDoneEvent {
        instance: instance_name,
        exit_code,
    }).ok();

    Ok(())
}
