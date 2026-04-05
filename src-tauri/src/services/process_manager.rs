use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use super::config_store::LaunchConfig;
use super::llama_detector;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    Server,
    Cli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub status: ProcessStatus,
    pub mode: Option<LaunchMode>,
    pub model: Option<String>,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
}

impl Default for ProcessInfo {
    fn default() -> Self {
        Self {
            status: ProcessStatus::Stopped,
            mode: None,
            model: None,
            port: None,
            pid: None,
            started_at: None,
        }
    }
}

pub struct ProcessManager {
    pub info: Arc<Mutex<ProcessInfo>>,
    child_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    child_pid: Arc<Mutex<Option<u32>>>,
    sp_temp_path: Arc<Mutex<Option<PathBuf>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            info: Arc::new(Mutex::new(ProcessInfo::default())),
            child_handle: Arc::new(Mutex::new(None)),
            child_pid: Arc::new(Mutex::new(None)),
            sp_temp_path: Arc::new(Mutex::new(None)),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    fn emit_status(&self, app: &AppHandle, info: &ProcessInfo) {
        app.emit("llama://status-change", info).ok();
    }

    pub async fn start(
        &self,
        app: AppHandle,
        llama_dir: &str,
        config: &LaunchConfig,
    ) -> Result<(), String> {
        // Store app handle for stop() to emit events
        *self.app_handle.lock().await = Some(app.clone());

        // Stop existing process first
        self.stop().await?;

        let binary = match config.mode {
            LaunchMode::Server => "llama-server",
            LaunchMode::Cli => "llama-cli",
        };

        let bin_path = llama_detector::get_binary_path(llama_dir, binary);
        if !bin_path.exists() {
            return Err(format!("找不到 {}: {}", binary, bin_path.display()));
        }

        let mut args: Vec<String> = Vec::new();

        // Model path (required)
        args.push("-m".into());
        args.push(config.model_path.clone());

        // GPU layers
        if let Some(ngl) = config.gpu_layers {
            args.push("-ngl".into());
            args.push(ngl.to_string());
        }

        // Context size
        if let Some(ctx) = config.ctx_size {
            args.push("-c".into());
            args.push(ctx.to_string());
        }

        // Threads
        if let Some(threads) = config.threads {
            args.push("-t".into());
            args.push(threads.to_string());
        }

        // 确定实际端口（0 或 None → 随机空闲端口）
        let actual_port = if matches!(config.mode, LaunchMode::Server) {
            let p = config.port.unwrap_or(0);
            Some(if p == 0 { find_free_port() } else { p })
        } else {
            None
        };

        // Server-specific args
        if matches!(config.mode, LaunchMode::Server) {
            let port = actual_port.unwrap();
            let host = config.host.clone().unwrap_or_else(|| "127.0.0.1".into());
            args.push("--host".into());
            args.push(host);
            args.push("--port".into());
            args.push(port.to_string());

            // --flash-attn requires on|off value (not a bare flag)
            args.push("--flash-attn".into());
            args.push(if config.flash_attn.unwrap_or(false) { "on" } else { "off" }.into());

            if config.cont_batching.unwrap_or(true) {
                args.push("--cont-batching".into());
            }

            // Batch size
            if let Some(b) = config.batch_size {
                args.push("-b".into());
                args.push(b.to_string());
            }
            // Ubatch size
            if let Some(ub) = config.ubatch_size {
                args.push("-ub".into());
                args.push(ub.to_string());
            }
            // Parallel decode slots
            if let Some(np) = config.parallel {
                args.push("--parallel".into());
                args.push(np.to_string());
            }
            // KV cache quantization type (K and V)
            if let Some(ref kt) = config.cache_type_k {
                if !kt.is_empty() {
                    args.push("--cache-type-k".into());
                    args.push(kt.clone());
                }
            }
            if let Some(ref vt) = config.cache_type_v {
                if !vt.is_empty() {
                    args.push("--cache-type-v".into());
                    args.push(vt.clone());
                }
            }
            // Keep KV cache entirely in VRAM (no CPU offload)
            if config.no_kv_offload.unwrap_or(false) {
                args.push("-nkvo".into());
            }
            // Seed
            if let Some(seed) = config.seed {
                args.push("--seed".into());
                args.push(seed.to_string());
            }
            // Memory-lock
            if config.mlock.unwrap_or(false) {
                args.push("--mlock".into());
            }
            // No memory-map
            if config.no_mmap.unwrap_or(false) {
                args.push("--no-mmap".into());
            }
            // API key
            if let Some(ref key) = config.api_key {
                if !key.is_empty() {
                    args.push("--api-key".into());
                    args.push(key.clone());
                }
            }
            // System prompt — write to uniquely-named temp file, pass via --system-prompt-file
            if let Some(ref sp) = config.system_prompt {
                if !sp.is_empty() {
                    let pid_hint = std::process::id();
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let filename = format!("llama_sp_{}_{}.txt", ts, pid_hint);
                    let tmp = std::env::temp_dir().join(&filename);
                    std::fs::write(&tmp, sp)
                        .map_err(|e| format!("写入系统提示词临时文件失败: {}", e))?;
                    *self.sp_temp_path.lock().await = Some(tmp.clone());
                    args.push("--system-prompt-file".into());
                    args.push(tmp.to_string_lossy().into_owned());
                }
            }
        }

        // CLI-specific args
        if matches!(config.mode, LaunchMode::Cli) {
            if let Some(ref prompt) = config.prompt {
                args.push("-p".into());
                args.push(prompt.clone());
            }
            if let Some(n) = config.predict {
                args.push("-n".into());
                args.push(n.to_string());
            }
        }

        // Extra args — shell-style splitting to handle quoted strings
        if let Some(ref extra) = config.extra_args {
            if !extra.trim().is_empty() {
                parse_shell_args(extra, &mut args)
                    .map_err(|e| format!("额外参数解析失败: {}", e))?;
            }
        }

        // Update status to Starting (hold lock for both update and emit)
        {
            let mut info = self.info.lock().await;
            info.status = ProcessStatus::Starting;
            info.mode = Some(config.mode);
            info.model = Some(config.model_path.clone());
            info.port = actual_port;
            self.emit_status(&app, &info);
        }

        // Spawn process (kill_on_drop ensures cleanup when Child is dropped)
        let mut cmd = Command::new(&bin_path);
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        // 防止在 Windows 上弹出控制台窗口（rpc-server.exe 等子进程同样生效）
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let mut child = cmd.spawn()
            .map_err(|e| format!("启动失败: {}", e))?;

        let pid = child.id();

        // Store PID for forceful kill on stop
        *self.child_pid.lock().await = pid;

        {
            let mut info = self.info.lock().await;
            info.pid = pid;
            info.started_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            // Keep status as Starting; log monitor will flip to Running when "listening" appears
            self.emit_status(&app, &info);
        }

        // Stream stdout/stderr in background task
        let info_clone = self.info.clone();
        let app_clone = app.clone();
        let is_server = matches!(config.mode, LaunchMode::Server);

        let handle = tokio::spawn(async move {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let stdout_task = tokio::spawn({
                let app = app_clone.clone();
                let info = info_clone.clone();
                async move {
                    if let Some(stdout) = stdout {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server).await;
                            app.emit("llama://log", &LogEvent { stream: "stdout".into(), line }).ok();
                        }
                    }
                }
            });

            let stderr_task = tokio::spawn({
                let app = app_clone.clone();
                let info = info_clone.clone();
                async move {
                    if let Some(stderr) = stderr {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server).await;
                            app.emit("llama://log", &LogEvent { stream: "stderr".into(), line }).ok();
                        }
                    }
                }
            });

            // Wait for process to finish
            let status = child.wait().await;
            stdout_task.await.ok();
            stderr_task.await.ok();

            // Update status on natural exit
            let mut info = info_clone.lock().await;
            info.status = match status {
                Ok(s) if s.success() => ProcessStatus::Stopped,
                _ => ProcessStatus::Error,
            };
            info.pid = None;
            info.started_at = None;

            app_clone.emit("llama://status-change", &*info).ok();
        });

        let mut handle_lock = self.child_handle.lock().await;
        *handle_lock = Some(handle);

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        // Abort the monitoring task — this drops the Child, triggering kill_on_drop
        if let Some(handle) = self.child_handle.lock().await.take() {
            handle.abort();
        }

        // Forcefully kill by PID to handle orphans
        if let Some(pid) = self.child_pid.lock().await.take() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string(), "/T"])
                    .output();
            }
            #[cfg(unix)]
            {
                unsafe { libc::kill(pid as i32, libc::SIGKILL); }
            }
        }

        // Clean up system prompt temp file
        if let Some(path) = self.sp_temp_path.lock().await.take() {
            let _ = std::fs::remove_file(path);
        }

        // Reset state — drop info lock before acquiring app_handle to avoid deadlock
        let was_running = {
            let mut info = self.info.lock().await;
            let was = info.status != ProcessStatus::Stopped;
            *info = ProcessInfo::default();
            was
        };
        if was_running {
            if let Some(app) = self.app_handle.lock().await.as_ref() {
                app.emit("llama://status-change", &ProcessInfo::default()).ok();
            }
        }

        Ok(())
    }

    pub async fn get_info(&self) -> ProcessInfo {
        self.info.lock().await.clone()
    }
}

#[derive(Clone, Serialize)]
struct LogEvent {
    stream: String,
    line: String,
}

/// Flip Starting → Running when the output line signals the server is listening.
/// Checks status before calling to_lowercase to avoid unnecessary allocation.
async fn maybe_flip_to_running(
    line: &str,
    info: &Arc<Mutex<ProcessInfo>>,
    app: &AppHandle,
    is_server: bool,
) {
    if !is_server {
        return;
    }
    let mut guard = info.lock().await;
    if guard.status != ProcessStatus::Starting {
        return;
    }
    let lower = line.to_lowercase();
    if lower.contains("listening") || lower.contains("server listening") {
        guard.status = ProcessStatus::Running;
        app.emit("llama://status-change", &*guard).ok();
    }
}

/// 让 OS 分配一个空闲端口（绑定后立即释放）
fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(18000)
}

/// Parse shell-style arguments, handling quoted strings.
/// Returns Err if there is an unclosed quote.
fn parse_shell_args(input: &str, args: &mut Vec<String>) -> Result<(), String> {
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if !in_single_quote => escape_next = true,
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_single_quote {
        return Err("额外参数中存在未闭合的单引号".into());
    }
    if in_double_quote {
        return Err("额外参数中存在未闭合的双引号".into());
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(())
}
