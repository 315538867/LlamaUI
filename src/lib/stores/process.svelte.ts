import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { InstanceMap, InstanceInfo, LogEvent } from "../types";
import { getAllInstances } from "../services/tauri-bridge";

let instances = $state<InstanceMap>({});

// Per-instance logs: name → log entries
let logs = $state<Record<string, { stream: string; line: string; ts: number }[]>>({});

// Last-seen perf metrics (from any instance)
let tokensPerSec = $state<number | null>(null);
let promptTps = $state<number | null>(null);

const RE_EVAL   = /(?:llama_print_timings|llama_perf_context_print):\s+eval time[^,]+,\s*([\d.]+)\s+tokens per second/;
const RE_PROMPT = /(?:llama_print_timings|llama_perf_context_print):\s+prompt eval time[^,]+,\s*([\d.]+)\s+tokens per second/;

function parsePerfLine(line: string) {
  const em = RE_EVAL.exec(line);
  if (em) { tokensPerSec = parseFloat(em[1]); return; }
  const pm = RE_PROMPT.exec(line);
  if (pm) { promptTps = parseFloat(pm[1]); }
}

let _initialized = false;
let _unlisteners: UnlistenFn[] = [];

export function getInstanceStore() {
  if (!_initialized) {
    _initialized = true;

    listen<InstanceMap>("llama://instances", (event) => {
      instances = event.payload;
    }).then((fn) => _unlisteners.push(fn));

    listen<LogEvent>("llama://log", (event) => {
      const { instance, stream, line } = event.payload;
      if (!logs[instance]) logs[instance] = [];
      const bucket = logs[instance];
      if (bucket.length >= 1000) logs[instance] = bucket.slice(500);
      logs[instance].push({ stream, line, ts: Date.now() });
      parsePerfLine(line);
    }).then((fn) => _unlisteners.push(fn));

    // Hydrate from backend on first load
    getAllInstances().then((map) => { instances = map; }).catch(() => {});
  }

  return {
    get instances() { return instances; },
    get logs() { return logs; },
    get tokensPerSec() { return tokensPerSec; },
    get promptTps() { return promptTps; },

    runningCount(): number {
      return Object.values(instances).filter((i: InstanceInfo) => i.status === "running").length;
    },
    getInstanceLogs(name: string) {
      return logs[name] ?? [];
    },
    clearLogs(name: string) {
      logs[name] = [];
    },
    destroy() {
      if (!_initialized) return;
      for (const fn of _unlisteners) fn();
      _unlisteners = [];
      _initialized = false;
    },
  };
}

// ── Backward-compat shim for StatusBar and other consumers ───────────────────

export function getProcessStore() {
  const store = getInstanceStore();
  return {
    get instances() { return store.instances; },
    get tokensPerSec() { return store.tokensPerSec; },
    get promptTps() { return store.promptTps; },
    runningCount: () => store.runningCount(),
    destroy: () => store.destroy(),
  };
}
