import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ModelInfo } from "../types";
import { scanModels, scanModelsStream } from "../services/tauri-bridge";

let models = $state<ModelInfo[]>([]);
let scanErrors = $state<string[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let _autoRefreshRegistered = false;
let _streamUnlisteners: UnlistenFn[] = [];

async function refreshStream() {
  if (loading) return;
  loading = true;
  error = null;
  scanErrors = [];
  models = [];

  // Clean up any previous listeners
  for (const fn of _streamUnlisteners) fn();
  _streamUnlisteners = [];

  const unFound = await listen<ModelInfo>("model://found", (event) => {
    models = [...models, event.payload];
  });
  _streamUnlisteners.push(unFound);

  const unDone = await listen<{ errors: string[] }>("model://scan-done", (event) => {
    scanErrors = event.payload.errors;
    loading = false;
    // Clean up listeners after scan completes
    for (const fn of _streamUnlisteners) fn();
    _streamUnlisteners = [];
  });
  _streamUnlisteners.push(unDone);

  try {
    await scanModelsStream();
  } catch (e) {
    error = String(e);
    loading = false;
    for (const fn of _streamUnlisteners) fn();
    _streamUnlisteners = [];
  }
}

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
    refreshStream();
  }

  return {
    get models() { return models; },
    get scanErrors() { return scanErrors; },
    get loading() { return loading; },
    get error() { return error; },
    refresh: refreshStream,
  };
}
