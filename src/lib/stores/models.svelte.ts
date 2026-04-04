import type { ModelInfo } from "../types";
import { scanModels } from "../services/tauri-bridge";

let models = $state<ModelInfo[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

export function getModelStore() {
  return {
    get models() {
      return models;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },
    async refresh() {
      loading = true;
      error = null;
      try {
        models = await scanModels();
      } catch (e) {
        error = String(e);
      } finally {
        loading = false;
      }
    },
  };
}
