import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  InstanceConfig,
  InstanceMap,
  LlamaInstall,
  ModelInfo,
  Preset,
  ScanResult,
} from "../types";

// ── Instance commands ─────────────────────────────────────────────────────────

export const startInstance = (config: InstanceConfig) =>
  invoke<void>("start_instance", { config });

export const stopInstance = (name: string) =>
  invoke<void>("stop_instance", { name });

export const getAllInstances = () =>
  invoke<InstanceMap>("get_all_instances");

export const deleteInstanceConfig = (name: string) =>
  invoke<void>("delete_instance_config", { name });

// ── Per-model preset commands ─────────────────────────────────────────────────

export const listModelPresets = (modelFilename: string) =>
  invoke<Preset[]>("list_model_presets", { modelFilename });

export const saveModelPreset = (modelFilename: string, preset: Preset) =>
  invoke<void>("save_model_preset", { modelFilename, preset });

export const deleteModelPreset = (modelFilename: string, name: string) =>
  invoke<void>("delete_model_preset", { modelFilename, name });

// ── Model commands ────────────────────────────────────────────────────────────

export const scanModels = () => invoke<ScanResult>("scan_models");

export const getModelInfo = (path: string) =>
  invoke<ModelInfo>("get_model_info", { path });

// ── Config commands ───────────────────────────────────────────────────────────

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

// ── Llama detection ───────────────────────────────────────────────────────────

export const detectLlama = () => invoke<LlamaInstall[]>("detect_llama");

export const validateLlamaPath = (path: string) =>
  invoke<LlamaInstall>("validate_llama_path", { path });

// ── Proxy commands ────────────────────────────────────────────────────────────

export const restartProxy = (
  port: number,
  cors: boolean,
  allowExternal: boolean,
  apiKey: string | null,
  responsesMode: 'direct' | 'anthropic',
) => invoke<void>("restart_proxy", { port, cors, allowExternal, apiKey, responsesMode });

export const getProxyStatus = () =>
  invoke<{
    running: boolean;
    port?: number;
    cors?: boolean;
    allow_external?: boolean;
    routes?: { name: string; port: number }[];
  }>("get_proxy_status");
