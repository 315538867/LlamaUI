import type { AppConfig, Preset } from "../types";
import { logger } from "../utils/logger";
import {
  getConfig,
  saveConfig as saveConfigApi,
  savePreset as savePresetApi,
  deletePreset as deletePresetApi,
  listPresets,
} from "../services/tauri-bridge";

let config = $state<AppConfig>({
  llama_dir: null,
  model_dirs: [],
  instances: [],
  model_presets: {},
  default_params: {
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
    system_prompt: null,
    extra_args: null,
    no_context_shift: null,
    keep: null,
  },
  last_preset: null,
  proxy_port: 8080,
  proxy_cors: true,
  proxy_allow_external: false,
  proxy_api_key: null,
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
        logger.error("Failed to load config:", e);
      }
    },

    async save(newConfig: AppConfig) {
      await saveConfigApi(newConfig);
      config = newConfig;
    },

    async savePreset(preset: Preset) {
      const prev = presets;
      // Optimistic update: replace in-place or append
      const idx = presets.findIndex((p) => p.name === preset.name);
      if (idx >= 0) {
        presets = presets.map((p, i) => (i === idx ? preset : p));
      } else {
        presets = [...presets, preset];
      }
      try {
        await savePresetApi(preset);
      } catch (e) {
        presets = prev; // rollback on failure
        throw e;
      }
    },

    async deletePreset(name: string) {
      const prev = presets;
      // Optimistic update: filter locally before server round-trip
      presets = presets.filter((p) => p.name !== name);
      try {
        await deletePresetApi(name);
      } catch (e) {
        presets = prev; // rollback on failure
        throw e;
      }
    },
  };
}
