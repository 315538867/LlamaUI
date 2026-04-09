use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

use crate::engine::Engine;
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
    #[allow(dead_code)]
    fn stopped(config: InstanceConfig) -> Self {
        Self { config, status: InstanceStatus::Stopped, port: None, pid: None, started_at: None }
    }
}

// ── Internal per-instance runtime state ──────────────────────────────────────

struct InstanceState {
    info: Arc<Mutex<InstanceInfo>>,
    child_handle: Option<tokio::task::JoinHandle<()>>,
    child_pid: Option<u32>,
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct InstanceRegistry {
    instances: Arc<Mutex<HashMap<String, InstanceState>>>,
    app_handle: std::sync::OnceLock<AppHandle>,
}

impl Default for InstanceRegistry {
    fn default() -> Self { Self::new() }
}

impl InstanceRegistry {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            app_handle: std::sync::OnceLock::new(),
        }
    }

    // ── Emit helpers ──────────────────────────────────────────────────────────

    async fn emit_instances(&self, app: &AppHandle) {
        emit_all_instances(&self.instances, app).await;
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Start a new instance. Returns the assigned port on success.
    pub async fn start(
        &self,
        app: AppHandle,
        llama_dir: &str,
        config: InstanceConfig,
    ) -> Result<u16, String> {
        self.app_handle.get_or_init(|| app.clone());

        // Stop any existing instance with the same name first
        self.stop(&config.name).await.ok();

        let engine = Engine::from_config(&config.mode);
        let binary = engine.binary_name(&config.mode);
        let bin_path = llama_detector::get_binary_path(llama_dir, binary);
        if !bin_path.exists() {
            return Err(format!("找不到 {}: {}", binary, bin_path.display()));
        }

        // Probe capabilities once — guards below use this to skip unsupported flags
        let caps = llama_detector::probe_capabilities(&bin_path);

        // Always random port for server mode
        let actual_port = if matches!(config.mode, LaunchMode::Server) {
            Some(find_free_port())
        } else {
            None
        };

        let args = engine.build_args(&config.model_path, &config.params, &caps, &config.mode, actual_port)?;

        // Build initial InstanceInfo
        let info = Arc::new(Mutex::new(InstanceInfo {
            config: config.clone(),
            status: InstanceStatus::Starting,
            port: actual_port,
            pid: None,
            started_at: None,
        }));

        // Emit starting status — register first so the snapshot includes this instance
        {
            let mut guard = self.instances.lock().await;
            guard.insert(config.name.clone(), InstanceState {
                info: Arc::clone(&info),
                child_handle: None,
                child_pid: None,
            });
        }
        self.emit_instances(&app).await;

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

            // Shared channel: stdout + stderr both send into this
            let (tx, mut rx) = mpsc::channel::<LogEntry>(256);

            let stdout_task = tokio::spawn({
                let tx = tx.clone();
                let app = app_clone.clone();
                let info = Arc::clone(&info_clone);
                let name = instance_name.clone();
                let instances = Arc::clone(&instances_ref);
                async move {
                    if let Some(stdout) = stdout {
                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server, &name, &instances, engine).await;
                            let _ = tx.send(LogEntry { stream: "stdout".into(), line }).await;
                        }
                    }
                }
            });

            let stderr_task = tokio::spawn({
                let tx = tx.clone();
                let app = app_clone.clone();
                let info = Arc::clone(&info_clone);
                let name = instance_name.clone();
                let instances = Arc::clone(&instances_ref);
                async move {
                    if let Some(stderr) = stderr {
                        let reader = BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            maybe_flip_to_running(&line, &info, &app, is_server, &name, &instances, engine).await;
                            let _ = tx.send(LogEntry { stream: "stderr".into(), line }).await;
                        }
                    }
                }
            });

            // Drop the original sender so channel closes when both reader tasks finish
            drop(tx);

            // Flusher: batch entries every 50ms or when buffer reaches 20
            let flush_app = app_clone.clone();
            let flush_name = instance_name.clone();
            let flusher = tokio::spawn(async move {
                let mut buf: Vec<LogEntry> = Vec::with_capacity(20);
                let mut interval = tokio::time::interval(Duration::from_millis(50));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        entry = rx.recv() => {
                            match entry {
                                Some(e) => {
                                    buf.push(e);
                                    if buf.len() >= 20 {
                                        if let Some(perf) = extract_perf_event(&flush_name, &buf) {
                                            flush_app.emit("llama://perf", &perf).ok();
                                        }
                                        flush_app.emit("llama://log/batch", &LogBatchEvent {
                                            instance: flush_name.clone(),
                                            entries: std::mem::take(&mut buf),
                                        }).ok();
                                    }
                                }
                                None => break, // channel closed
                            }
                        }
                        _ = interval.tick() => {
                            if !buf.is_empty() {
                                if let Some(perf) = extract_perf_event(&flush_name, &buf) {
                                    flush_app.emit("llama://perf", &perf).ok();
                                }
                                flush_app.emit("llama://log/batch", &LogBatchEvent {
                                    instance: flush_name.clone(),
                                    entries: std::mem::take(&mut buf),
                                }).ok();
                            }
                        }
                    }
                }
                // Final flush — emit any remaining entries after channel closes
                if !buf.is_empty() {
                    if let Some(perf) = extract_perf_event(&flush_name, &buf) {
                        flush_app.emit("llama://perf", &perf).ok();
                    }
                    flush_app.emit("llama://log/batch", &LogBatchEvent {
                        instance: flush_name.clone(),
                        entries: buf,
                    }).ok();
                }
            });

            let status = child.wait().await;
            stdout_task.await.ok();
            stderr_task.await.ok();
            flusher.await.ok();

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
            emit_all_instances(&instances_ref, &app_clone).await;
        });

        // Update registry entry with the live child handle (inserted earlier as placeholder)
        {
            let mut guard = self.instances.lock().await;
            if let Some(state) = guard.get_mut(&config.name) {
                state.child_handle = Some(handle);
                state.child_pid = pid;
            }
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

        if let Some(app) = self.app_handle.get() {
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
        let info_refs: Vec<Arc<Mutex<InstanceInfo>>> = {
            let guard = self.instances.lock().await;
            guard.values().map(|s| Arc::clone(&s.info)).collect()
        };
        let mut map = HashMap::with_capacity(info_refs.len());
        for info in &info_refs {
            let snapshot = info.lock().await.clone();
            map.insert(snapshot.config.name.clone(), snapshot);
        }
        map
    }
}

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub stream: String,
    pub line: String,
}

#[derive(Clone, Serialize)]
pub struct LogBatchEvent {
    pub instance: String,
    pub entries: Vec<LogEntry>,
}

#[derive(Clone, Serialize)]
pub struct PerfEvent {
    pub instance: String,
    pub eval_tps: Option<f64>,
    pub prompt_tps: Option<f64>,
    pub eval_tokens: Option<u32>,
    pub prompt_tokens: Option<u32>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn maybe_flip_to_running(
    line: &str,
    info: &Arc<Mutex<InstanceInfo>>,
    app: &AppHandle,
    is_server: bool,
    _instance_name: &str,
    instances: &Arc<Mutex<HashMap<String, InstanceState>>>,
    engine: Engine,
) {
    if !is_server { return; }
    let mut guard = info.lock().await;
    if guard.status != InstanceStatus::Starting { return; }
    let bytes = line.as_bytes();
    let pattern = engine.ready_pattern();
    let found = bytes.windows(pattern.len()).any(|w| w.eq_ignore_ascii_case(pattern));
    if found {
        guard.status = InstanceStatus::Running;
        drop(guard);
        emit_all_instances(instances, app).await;
    }
}

fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(18000)
}

// ── Shared emit helper (usable inside spawn closures) ────────────────────────

async fn emit_all_instances(
    instances: &Arc<Mutex<HashMap<String, InstanceState>>>,
    app: &AppHandle,
) {
    // 先收集所有 info Arc，释放外层锁，再逐个读取子锁，避免嵌套持锁
    let info_refs: Vec<Arc<Mutex<InstanceInfo>>> = {
        let guard = instances.lock().await;
        guard.values().map(|s| Arc::clone(&s.info)).collect()
    };
    let mut map: HashMap<String, InstanceInfo> = HashMap::with_capacity(info_refs.len());
    for info in &info_refs {
        let snapshot = info.lock().await.clone();
        map.insert(snapshot.config.name.clone(), snapshot);
    }
    app.emit("llama://instances", &map).ok();
}

// ── Perf-line parsing helpers ─────────────────────────────────────────────────

/// Scan a batch of log entries for timing lines; return a `PerfEvent` if found.
fn extract_perf_event(instance: &str, buf: &[LogEntry]) -> Option<PerfEvent> {
    let mut eval_tps: Option<f64> = None;
    let mut prompt_tps: Option<f64> = None;
    let mut eval_tokens: Option<u32> = None;
    let mut prompt_tokens: Option<u32> = None;

    for entry in buf {
        let line = &entry.line;
        if line.contains("prompt eval time") {
            if let Some((tok, tps)) = parse_timing_line(line) {
                prompt_tokens = Some(tok);
                prompt_tps = Some(tps);
            }
        } else if line.contains("eval time") {
            if let Some((tok, tps)) = parse_timing_line(line) {
                eval_tokens = Some(tok);
                eval_tps = Some(tps);
            }
        }
    }

    if eval_tps.is_some() || prompt_tps.is_some() {
        Some(PerfEvent {
            instance: instance.to_string(),
            eval_tps,
            prompt_tps,
            eval_tokens,
            prompt_tokens,
        })
    } else {
        None
    }
}

/// Extract (tokens, tps) from a timing line of the form:
///   "… / 769 tokens (…, 63.68 tokens per second)"
fn parse_timing_line(line: &str) -> Option<(u32, f64)> {
    // Token count: find "/ " then parse until " tokens"
    let slash_pos = line.find("/ ")?;
    let after_slash = &line[slash_pos + 2..];
    let tok_end = after_slash.find(" tokens")?;
    let tokens: u32 = after_slash[..tok_end].trim().parse().ok()?;

    // TPS: find last ", " then parse until " tokens per second"
    let comma_pos = line.rfind(", ")?;
    let after_comma = &line[comma_pos + 2..];
    let tps_end = after_comma.find(" tokens per second")?;
    let tps: f64 = after_comma[..tps_end].trim().parse().ok()?;

    Some((tokens, tps))
}
