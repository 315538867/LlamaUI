import type { ModelInfo } from "../types";
import { scanModels } from "../services/tauri-bridge";

let models = $state<ModelInfo[]>([]);
let scanErrors = $state<string[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let _autoRefreshRegistered = false;

async function refresh() {
  if (loading) return;
  loading = true;
  error = null;
  scanErrors = [];
  try {
    const result = await scanModels();
    models = result.models;
    scanErrors = result.scan_errors;
  } catch (e) {
    error = String(e);
  } finally {
    loading = false;
  }
}

export function getModelStore() {
  if (!_autoRefreshRegistered) {
    _autoRefreshRegistered = true;
    refresh();
  }

  return {
    get models() {
      return models;
    },
    get scanErrors() {
      return scanErrors;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },
    refresh,
  };
}
