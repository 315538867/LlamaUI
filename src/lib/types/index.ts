// ── Launch params ─────────────────────────────────────────────────────────────

export type LaunchMode = "server" | "cli";

export interface LaunchParams {
  gpu_layers: number | null;
  ctx_size: number | null;
  threads: number | null;
  flash_attn: boolean | null;
  cont_batching: boolean | null;
  batch_size: number | null;
  ubatch_size: number | null;
  parallel: number | null;
  cache_type_k: string | null;
  cache_type_v: string | null;
  no_kv_offload: boolean | null;
  seed: number | null;
  mlock: boolean | null;
  no_mmap: boolean | null;
  extra_args: string | null;
  no_context_shift: boolean | null;
  keep: number | null;
}

// ── Instance types ────────────────────────────────────────────────────────────

export interface InstanceConfig {
  name: string;         // routing key (= body.model in Codex requests)
  model_path: string;
  mode: LaunchMode;
  params: LaunchParams;
}

export type InstanceStatus = "stopped" | "starting" | "running" | "error";

export interface InstanceInfo {
  config: InstanceConfig;         // nested, mirrors Rust struct
  status: InstanceStatus;
  port: number | null;
  pid: number | null;
  started_at: number | null;
}

export type InstanceMap = Record<string, InstanceInfo>;

// ── Log events ────────────────────────────────────────────────────────────────

export interface LogEvent {
  instance: string;
  stream: "stdout" | "stderr";
  line: string;
}

export interface ProxyLogEvent {
  timestamp: number;
  level: "info" | "warn" | "error";
  message: string;
}

// ── Model types ───────────────────────────────────────────────────────────────

export interface ModelInfo {
  name: string;
  path: string;
  size_bytes: number;
  size_display: string;
  quantization: string | null;
  modified: number | null;
}

export interface ScanResult {
  models: ModelInfo[];
  scan_errors: string[];
}

// ── Config types ──────────────────────────────────────────────────────────────

export interface AppConfig {
  llama_dir: string | null;
  model_dirs: string[];
  instances: InstanceConfig[];
  model_presets: Record<string, Preset[]>;
  default_params: LaunchParams;
  last_preset: string | null;
  proxy_port: number;
  proxy_cors: boolean;
  proxy_allow_external: boolean;
  proxy_api_key: string | null;
  proxy_responses_mode: 'direct' | 'anthropic';
}

export interface Preset {
  name: string;
  params: LaunchParams;
  mode: LaunchMode;
}

export interface LlamaInstall {
  path: string;
  version: string | null;
  has_server: boolean;
  has_cli: boolean;
}

// ── UI types ──────────────────────────────────────────────────────────────────

export type PageId = "instances" | "models" | "proxy" | "settings";


// ── Constants ─────────────────────────────────────────────────────────────────

export const DEFAULT_PARAMS: LaunchParams = {
  gpu_layers: 99,
  ctx_size: 4096,
  threads: null,
  flash_attn: true,
  cont_batching: true,
  batch_size: null,
  ubatch_size: null,
  parallel: null,
  cache_type_k: null,
  cache_type_v: null,
  no_kv_offload: null,
  seed: null,
  mlock: null,
  no_mmap: null,
  extra_args: null,
  no_context_shift: null,
  keep: null,
};

export const PARAM_LABELS: Record<string, string> = {
  gpu_layers: "GPU 层数",
  ctx_size: "上下文长度",
  threads: "线程数",
  flash_attn: "Flash Attention",
  cont_batching: "连续批处理",
  batch_size: "批处理大小",
  ubatch_size: "微批大小",
  parallel: "并行槽",
  cache_type_k: "KV缓存K类型",
  cache_type_v: "KV缓存V类型",
  no_kv_offload: "禁用KV卸载",
  seed: "随机种子",
  mlock: "内存锁定",
  no_mmap: "禁用内存映射",
  no_context_shift: "禁用上下文移位",
  keep: "保留头部Token数",
  extra_args: "额外参数",
};
