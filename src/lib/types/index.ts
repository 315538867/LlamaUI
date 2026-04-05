// Process types
export type ProcessStatus = "stopped" | "starting" | "running" | "error";
export type LaunchMode = "server" | "cli";

export interface ProcessInfo {
  status: ProcessStatus;
  mode: LaunchMode | null;
  model: string | null;
  port: number | null;
  pid: number | null;
  started_at: number | null;
}

export interface LaunchConfig {
  model_path: string;
  mode: LaunchMode;
  gpu_layers?: number;
  ctx_size?: number;
  threads?: number;
  flash_attn?: boolean;
  cont_batching?: boolean;
  batch_size?: number;
  ubatch_size?: number;
  parallel?: number;
  cache_type_k?: string;
  cache_type_v?: string;
  no_kv_offload?: boolean;
  seed?: number;
  mlock?: boolean;
  no_mmap?: boolean;
  api_key?: string;
  system_prompt?: string;
  prompt?: string;
  predict?: number;
  extra_args?: string;
}

// Model types
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

// Config types
export interface AppConfig {
  llama_dir: string | null;
  model_dirs: string[];
  default_params: LaunchParams;
  last_preset: string | null;
  proxy_port: number;
  proxy_cors: boolean;
  proxy_allow_external: boolean;
}

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
  api_key: string | null;
  system_prompt: string | null;
  extra_args: string | null;
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

export interface LogEvent {
  stream: "stdout" | "stderr";
  line: string;
}

// UI types
export type PageId = "launcher" | "models" | "settings";
