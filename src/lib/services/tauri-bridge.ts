import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  LaunchConfig,
  LlamaInstall,
  ModelInfo,
  Preset,
  ProcessInfo,
} from "../types";

// Process commands
export const startLlama = (config: LaunchConfig) =>
  invoke<void>("start_llama", { config });

export const stopLlama = () => invoke<void>("stop_llama");

export const getLlamaStatus = () => invoke<ProcessInfo>("get_llama_status");

// Model commands
export const scanModels = () => invoke<ModelInfo[]>("scan_models");

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
