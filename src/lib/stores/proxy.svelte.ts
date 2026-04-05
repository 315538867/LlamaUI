import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ProxyLogEvent } from "../types";

let proxyLogs = $state<ProxyLogEvent[]>([]);
let _initialized = false;
let _unlisteners: UnlistenFn[] = [];

export function getProxyStore() {
  if (!_initialized) {
    _initialized = true;

    listen<ProxyLogEvent>("proxy://log", (event) => {
      if (proxyLogs.length >= 500) proxyLogs = proxyLogs.slice(250);
      proxyLogs.push(event.payload);
    }).then((fn) => _unlisteners.push(fn));
  }

  return {
    get logs() { return proxyLogs; },
    clearLogs() { proxyLogs = []; },
    destroy() {
      if (!_initialized) return;
      for (const fn of _unlisteners) fn();
      _unlisteners = [];
      _initialized = false;
    },
  };
}
