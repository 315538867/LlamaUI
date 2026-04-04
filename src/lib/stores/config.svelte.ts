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
    port: 8080,
    host: "127.0.0.1",
    flash_attn: true,
    cont_batching: true,
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
