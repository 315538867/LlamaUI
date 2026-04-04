import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ProcessInfo, LogEvent } from "../types";
import { getLlamaStatus } from "../services/tauri-bridge";

let processInfo = $state<ProcessInfo>({
  status: "stopped",
  mode: null,
  model: null,
  port: null,
  pid: null,
  started_at: null,
});

let logs = $state<{ stream: string; line: string; ts: number }[]>([]);

let _initialized = false;
let _unlisteners: UnlistenFn[] = [];

export function getProcessStore() {
  if (!_initialized) {
    _initialized = true;

    // Store unlisten functions to prevent listener leak (#11 fix)
    listen<ProcessInfo>("llama://status-change", (event) => {
      processInfo = event.payload;
    }).then((fn) => _unlisteners.push(fn));

    listen<LogEvent>("llama://log", (event) => {
      // In-place splice instead of slice for efficiency (#12 fix)
      if (logs.length >= 2000) {
        logs.splice(0, 500);
      }
      logs.push({ ...event.payload, ts: Date.now() });
    }).then((fn) => _unlisteners.push(fn));

    // Initial fetch
    getLlamaStatus().then((info) => {
      processInfo = info;
    });
  }

  return {
    get info() {
      return processInfo;
    },
    get logs() {
      return logs;
    },
    get isRunning() {
      return processInfo.status === "running";
    },
    get isStarting() {
      return processInfo.status === "starting";
    },
    clearLogs() {
      logs = [];
    },
    destroy() {
      for (const fn of _unlisteners) fn();
      _unlisteners = [];
      _initialized = false;
    },
  };
}
