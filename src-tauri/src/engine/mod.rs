use crate::services::config_store::{LaunchMode, LaunchParams};
use crate::services::llama_detector::LlamaCapabilities;

// ── Engine enum ───────────────────────────────────────────────────────────────

/// Dispatch tag for the backend engine. `Copy` so it can be captured into
/// tokio::spawn closures without wrapping in Arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    LlamaServer,
}

/// API surface a running engine exposes.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct EngineApiCaps {
    pub has_chat: bool,
    pub has_completions: bool,
    pub has_embeddings: bool,
    pub has_responses: bool,
}

impl Engine {
    /// Select engine from instance launch mode. Currently always LlamaServer;
    /// future variants (MLX, etc.) will add branches here.
    pub fn from_config(_mode: &LaunchMode) -> Self {
        Engine::LlamaServer
    }

    /// Binary name for the given launch mode.
    pub fn binary_name(&self, mode: &LaunchMode) -> &'static str {
        match self {
            Engine::LlamaServer => match mode {
                LaunchMode::Server => "llama-server",
                LaunchMode::Cli => "llama-cli",
            },
        }
    }

    /// Byte sequence that signals the server is ready to accept connections.
    pub fn ready_pattern(&self) -> &'static [u8] {
        match self {
            Engine::LlamaServer => b"listening",
        }
    }

    /// Build the full argument list for launching this engine.
    pub fn build_args(
        &self,
        model_path: &str,
        params: &LaunchParams,
        caps: &LlamaCapabilities,
        mode: &LaunchMode,
        port: Option<u16>,
    ) -> Result<Vec<String>, String> {
        match self {
            Engine::LlamaServer => {
                build_llama_args(model_path, params, caps, mode, port)
            }
        }
    }

    /// Append speculative-decoding arguments. Kept as a separate public method
    /// so MCP server can call it after building its own minimal base arg list.
    pub fn append_draft_args(
        &self,
        args: &mut Vec<String>,
        params: &LaunchParams,
        caps: &LlamaCapabilities,
    ) {
        match self {
            Engine::LlamaServer => append_llama_draft_args(args, params, caps),
        }
    }

    /// API capabilities exposed by this engine.
    #[allow(dead_code)]
    pub fn api_capabilities(&self) -> EngineApiCaps {
        match self {
            Engine::LlamaServer => EngineApiCaps {
                has_chat: true,
                has_completions: true,
                has_embeddings: true,
                has_responses: true,
            },
        }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn build_llama_args(
    model_path: &str,
    p: &LaunchParams,
    caps: &LlamaCapabilities,
    mode: &LaunchMode,
    port: Option<u16>,
) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = Vec::new();

    // Model
    args.push("-m".into());
    args.push(model_path.to_string());

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

    // Server-specific args
    if matches!(mode, LaunchMode::Server) {
        let p_val = port.ok_or("Server mode requires a port")?;
        args.push("--host".into());
        args.push("127.0.0.1".into());
        args.push("--port".into());
        args.push(p_val.to_string());

        if caps.supports_flash_attn {
            args.push("--flash-attn".into());
            args.push(if p.flash_attn.unwrap_or(false) { "on" } else { "off" }.into());
        }

        if caps.supports_cont_batching && p.cont_batching.unwrap_or(true) {
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
        if caps.supports_kv_quant {
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
        if p.no_context_shift.unwrap_or(false) {
            args.push("--no-context-shift".into());
        }
        if let Some(k) = p.keep {
            args.push("--keep".into());
            args.push(k.to_string());
        }
    }

    // Extra args
    if let Some(ref extra) = p.extra_args {
        if !extra.trim().is_empty() {
            parse_shell_args(extra, &mut args)
                .map_err(|e| format!("额外参数解析失败: {}", e))?;
        }
    }

    // Speculative decoding
    append_llama_draft_args(&mut args, p, caps);

    Ok(args)
}

fn append_llama_draft_args(args: &mut Vec<String>, params: &LaunchParams, caps: &LlamaCapabilities) {
    if let Some(ref draft) = params.model_draft {
        if !draft.is_empty() && caps.supports_speculative {
            args.push("--model-draft".into());
            args.push(draft.clone());
            if let Some(ngld) = params.gpu_layers_draft {
                args.push("-ngld".into());
                args.push(ngld.to_string());
            }
            if let Some(dm) = params.draft_max {
                args.push("--draft".into());
                args.push(dm.to_string());
            }
            if let Some(dmin) = params.draft_min {
                args.push("--draft-min".into());
                args.push(dmin.to_string());
            }
            if let Some(dpmin) = params.draft_p_min {
                args.push("--draft-p-min".into());
                args.push(dpmin.to_string());
            }
            if let Some(cd) = params.ctx_size_draft {
                args.push("-cd".into());
                args.push(cd.to_string());
            }
        }
    }
    // --spec-type has its own capability flag (independent of model-draft support)
    if let Some(ref st) = params.spec_type {
        if !st.is_empty() && caps.supports_spec_type {
            args.push("--spec-type".into());
            args.push(st.clone());
        }
    }
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
