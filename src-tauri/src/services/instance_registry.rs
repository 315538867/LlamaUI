use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

use super::config_store::{InstanceConfig, LaunchMode};
use super::llama_detector;

// ── Public status/info types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Serializable snapshot of a single instance's runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub config: InstanceConfig,
    pub status: InstanceStatus,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
}

impl InstanceInfo {
    fn stopped(config: InstanceConfig) -> Self {
        Self { config, status: InstanceStatus::Stopped, port: None, pid: None, started_at: None }
    }
}

// ── Internal per-instance runtime state ──────────────────────────────────────

struct InstanceState {
    info: Arc<Mutex<InstanceInfo>>,
    child_handle: Option<tokio::task::JoinHandle<()>>,
    child_pid: Option<u32>,
    sp_temp_path: Option<PathBuf>,
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct InstanceRegistry {
    instances: Arc<Mutex<HashMap<String, InstanceState>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

impl Default for InstanceRegistry {
    fn default() -> Self { Self::new() }
}

impl InstanceRegistry {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    // ── Emit helpers ──────────────────────────────────────────────────────────

    async fn emit_instances(&self, app: &AppHandle) {
        let snapshot = self.get_all().await;
        app.emit("llama://instances", &snapshot).ok();
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Start a new instance. Returns the assigned port on success.
    pub async fn start(
        &self,
        app: AppHandle,
        llama_dir: &str,
        config: InstanceConfig,
    ) -> Result<u16, String> {
        *self.app_handle.lock().await = Some(app.clone());

        // Stop any existing instance with the same name first
        self.stop(&config.name).await.ok();

        let binary = match config.mode {
            LaunchMode::Server => "llama-server",
            LaunchMode::Cli => "llama-cli",
        };

        let bin_path = llama_detector::get_binary_path(llama_dir, binary);
        if !bin_path.exists() {
            return Err(format!("找不到 {}: {}", binary, bin_path.display()));
        }

        let mut args: Vec<String> = Vec::new();
        let p = &config.params;

        // Model
        args.push("-m".into());
        args.push(config.model_path.clone());

        // GPU layers
        if let Some(ngl) = p.gpu_layers {
            args.push("-ngl".into());
            args.push(ngl.to_string());
        }
        // Context size
        if let Some(ctx) = p.ctx_size {
            args.push("-c".into());
            args.push(ctx.to_string());
        }
        // Threads
        if let Some(t) = p.threads {
            if t > 0 {
                args.push("-t".into());
                args.push(t.to_string());
            }
        }

        // Always random port for server mode
        let actual_port = if matches!(config.mode, LaunchMode::Server) {
            Some(find_free_port())
        } else {
            None
        };

        // Server-specific args
        if matches!(config.mode, LaunchMode::Server) {
            let port = actual_port.unwrap();
            args.push("--host".into());
            args.push("127.0.0.1".into());
            args.push("--port".into());
            args.push(port.to_string());

            args.push("--flash-attn".into());
            args.push(if p.flash_attn.unwrap_or(false) { "on" } else { "off" }.into());

            if p.cont_batching.unwrap_or(true) {
                args.push("--cont-batching".into());
            }
            if let Some(b) = p.batch_size {
                args.push("-b".into());
                args.push(b.to_string());
            }
            if let Some(ub) = p.ubatch_size {
                args.push("-ub".into());
                args.push(ub.to_string());
            }
            if let Some(np) = p.parallel {
                args.push("--parallel".into());
                args.push(np.to_string());
            }
            if let Some(ref kt) = p.cache_type_k {
                if !kt.is_empty() {
                    args.push("--cache-type-k".into());
                    args.push(kt.clone());
                }
            }
            if let Some(ref vt) = p.cache_type_v {
                if !vt.is_empty() {
                    args.push("--cache-type-v".into());
                    args.push(vt.clone());
                }
            }
            if p.no_kv_offload.unwrap_or(false) {
                args.push("-nkvo".into());
            }
            if let Some(seed) = p.seed {
                args.push("--seed".into());
                args.push(seed.to_string());
            }
            if p.mlock.unwrap_or(false) {
                args.push("--mlock".into());
            }
            if p.no_mmap.unwrap_or(false) {
                args.push("--no-mmap".into());
            }
            // Instance-level API key (protects direct llama.cpp access)
            if let Some(ref key) = p.api_key {
                if !key.is_empty() {
                    args.push("--api-key".into());
                    args.push(key.clone());
                }
            }
        }

        // System prompt → temp file
        let mut sp_temp_path: Option<PathBuf> = None;
        if matches!(config.mode, LaunchMode::Server) {
            if let Some(ref sp) = p.system_prompt {
                if !sp.is_empty() {
                    let pid_hint = std::process::id();
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let tmp = std::env::temp_dir()
                        .join(format!("llama_sp_{}_{}.txt", ts, pid_hint));
                    std::fs::write(&tmp, sp)
                        .map_err(|e| format!("写入系统提示词临时文件失败: {}", e))?;
                    args.push("--system-prompt-file".into());
                    args.push(tmp.to_string_lossy().into_owned());
                    sp_temp_path = Some(tmp);
                }
            }
        }

        // Extra args
        if let Some(ref extra) = p.extra_args {
            if !extra.trim().is_empty() {
                parse_shell_args(extra, &mut args)
                    .map_err(|e| format!("额外参数解析失败: {}", e))?;
            }
        }

        // Build initial InstanceInfo
        let info = Arc::new(Mutex::new(InstanceInfo {
            config: config.clone(),
            status: InstanceStatus::Starting,
            port: actual_port,
            pid: None,
            started_at: None,
        }));

        // Emit starting status
        {
            let snapshot = info.lock().await.clone();
            app.emit("llama://instances", &{
                let mut m = HashMap::new();
                m.insert(config.name.clone(), snapshot);
                m
            }).ok();
        }

        // Spawn process
        let mut cmd = Command::new(&bin_path);
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let mut child = cmd.spawn()
            .map_err(|e| format!("启动失败: {}", e))?;

        let pid = child.id();

        {
            let mut i = info.lock().await;
            i.pid = pid;
            i.started_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }

        // Background log streaming
        let info_clone = Arc::clone(&info);
        let app_clone = app.clone();
        let instance_name = config.name.clone();
        let is_server = matches!(config.mode, LaunchMode::Server);
        let instances_ref = Arc::clone(&self.instances);

        let handle = tokio::spawn(async move {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let stdout_task = tokio::spawn({
                let app = app_clone.clone();
                let info = Arc::clone(&info_clone);
                let name = instance_name.clone();
                async move {
                    if let Some(stdout) = stdout {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server, &name).await;
                            app.emit("llama://log", &LogEvent {
                                instance: name.clone(), stream: "stdout".into(), line,
                            }).ok();
                        }
                    }
                }
            });

            let stderr_task = tokio::spawn({
                let app = app_clone.clone();
                let info = Arc::clone(&info_clone);
                let name = instance_name.clone();
                async move {
                    if let Some(stderr) = stderr {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server, &name).await;
                            app.emit("llama://log", &LogEvent {
                                instance: name.clone(), stream: "stderr".into(), line,
                            }).ok();
                        }
                    }
                }
            });

            let status = child.wait().await;
            stdout_task.await.ok();
            stderr_task.await.ok();

            // Update final status
            {
                let mut i = info_clone.lock().await;
                i.status = match status {
                    Ok(s) if s.success() => InstanceStatus::Stopped,
                    _ => InstanceStatus::Error,
                };
                i.pid = None;
                i.started_at = None;
            }

            // Emit full instances snapshot
            let snapshot = {
                let guard = instances_ref.lock().await;
                let mut map: HashMap<String, InstanceInfo> = HashMap::new();
                for (k, v) in guard.iter() {
                    map.insert(k.clone(), v.info.lock().await.clone());
                }
                map
            };
            app_clone.emit("llama://instances", &snapshot).ok();
        });

        // Register in registry
        {
            let mut guard = self.instances.lock().await;
            guard.insert(config.name.clone(), InstanceState {
                info,
                child_handle: Some(handle),
                child_pid: pid,
                sp_temp_path,
            });
        }

        // Emit full snapshot
        self.emit_instances(&app).await;

        Ok(actual_port.unwrap_or(0))
    }

    /// Stop a named instance.
    pub async fn stop(&self, name: &str) -> Result<(), String> {
        let mut guard = self.instances.lock().await;
        let Some(state) = guard.get_mut(name) else {
            return Ok(()); // Already gone
        };

        if let Some(h) = state.child_handle.take() {
            h.abort();
        }
        if let Some(pid) = state.child_pid.take() {
            #[cfg(windows)]
            { let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string(), "/T"]).output(); }
            #[cfg(unix)]
            { unsafe { libc::kill(pid as i32, libc::SIGKILL); } }
        }
        if let Some(path) = state.sp_temp_path.take() {
            let _ = std::fs::remove_file(path);
        }

        // Update status to stopped
        {
            let mut i = state.info.lock().await;
            i.status = InstanceStatus::Stopped;
            i.pid = None;
            i.port = None;
            i.started_at = None;
        }

        // Remove from registry (stopped instances are removed)
        guard.remove(name);
        drop(guard);

        if let Some(app) = self.app_handle.lock().await.as_ref() {
            self.emit_instances(app).await;
        }
        Ok(())
    }

    /// Stop all running instances (used on window close).
    pub async fn stop_all(&self) {
        let names: Vec<String> = {
            self.instances.lock().await.keys().cloned().collect()
        };
        for name in names {
            self.stop(&name).await.ok();
        }
    }

    /// Snapshot of all current instances (including stopped ones stored in config).
    pub async fn get_all(&self) -> HashMap<String, InstanceInfo> {
        let guard = self.instances.lock().await;
        let mut map = HashMap::new();
        for (k, v) in guard.iter() {
            map.insert(k.clone(), v.info.lock().await.clone());
        }
        map
    }
}

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct LogEvent {
    pub instance: String,
    pub stream: String,
    pub line: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn maybe_flip_to_running(
    line: &str,
    info: &Arc<Mutex<InstanceInfo>>,
    app: &AppHandle,
    is_server: bool,
    instance_name: &str,
) {
    if !is_server { return; }
    let mut guard = info.lock().await;
    if guard.status != InstanceStatus::Starting { return; }
    let lower = line.to_lowercase();
    if lower.contains("listening") || lower.contains("server listening") {
        guard.status = InstanceStatus::Running;
        app.emit("llama://instance-running", instance_name).ok();
    }
}

fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(18000)
}

fn parse_shell_args(input: &str, args: &mut Vec<String>) -> Result<(), String> {
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;

    for ch in input.chars() {
        if escape { current.push(ch); escape = false; continue; }
        match ch {
            '\\' if !in_single => escape = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() { args.push(std::mem::take(&mut current)); }
            }
            _ => current.push(ch),
        }
    }
    if in_single { return Err("额外参数中存在未闭合的单引号".into()); }
    if in_double { return Err("额外参数中存在未闭合的双引号".into()); }
    if !current.is_empty() { args.push(current); }
    Ok(())
}
