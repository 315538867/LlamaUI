import type { AppConfig, Preset } from "../types";
import {
  getConfig,
  saveConfig as saveConfigApi,
  listPresets,
  savePreset as savePresetApi,
  deletePreset as deletePresetApi,
} from "../services/tauri-bridge";

let config = $state<AppConfig>({
  llama_dir: null,
  model_dirs: [],
  default_params: {
    gpu_layers: 99,
    ctx_size: 4096,
    threads: null,
    port: 8000,
    host: "127.0.0.1",
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
    api_key: null,
    cors_allow_origins: null,
    system_prompt: null,
    extra_args: null,
  },
  last_preset: null,
});

let presets = $state<Preset[]>([]);
let loaded = $state(false);
let loadError = $state<string | null>(null);

export function getConfigStore() {
  return {
    get config() {
      return config;
    },
    get presets() {
      return presets;
    },
    get loaded() {
      return loaded;
    },
    get loadError() {
      return loadError;
    },

    async load() {
      loadError = null;
      try {
        config = await getConfig();
        presets = await listPresets();
        loaded = true;
      } catch (e) {
        loadError = String(e);
        console.error("Failed to load config:", e);
      }
    },

    async save(newConfig: AppConfig) {
      await saveConfigApi(newConfig);
      config = newConfig;
    },

    async savePreset(preset: Preset) {
      await savePresetApi(preset);
      presets = await listPresets();
    },

    async deletePreset(name: string) {
      await deletePresetApi(name);
      presets = await listPresets();
    },
  };
}
