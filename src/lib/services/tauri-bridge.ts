import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  LaunchConfig,
  LlamaInstall,
  ModelInfo,
  ScanResult,
  Preset,
  ProcessInfo,
} from "../types";

// Process commands
export const startLlama = (config: LaunchConfig) =>
  invoke<void>("start_llama", { config });

export const stopLlama = () => invoke<void>("stop_llama");

export const getLlamaStatus = () => invoke<ProcessInfo>("get_llama_status");

// Model commands
export const scanModels = () => invoke<ScanResult>("scan_models");

export const getModelInfo = (path: string) =>
  invoke<ModelInfo>("get_model_info", { path });

// Config commands
export const getConfig = () => invoke<AppConfig>("get_config");

export const saveConfig = (config: AppConfig) =>
  invoke<void>("save_config", { config });

export const listPresets = () => invoke<Preset[]>("list_presets");

export const savePreset = (preset: Preset) =>
  invoke<void>("save_preset", { preset });

export const loadPreset = (name: string) =>
  invoke<Preset>("load_preset", { name });

export const deletePreset = (name: string) =>
  invoke<void>("delete_preset", { name });

// Llama detection
export const detectLlama = () => invoke<LlamaInstall[]>("detect_llama");

export const validateLlamaPath = (path: string) =>
  invoke<LlamaInstall>("validate_llama_path", { path });

// Proxy commands
export const restartProxy = (port: number, cors: boolean, allowExternal: boolean) =>
  invoke<void>("restart_proxy", { port, cors, allowExternal });

export const getProxyStatus = () =>
  invoke<{ running: boolean; port?: number; cors?: boolean; allow_external?: boolean; target?: string }>("get_proxy_status");
