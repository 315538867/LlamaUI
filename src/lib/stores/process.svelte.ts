import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { InstanceMap, InstanceInfo, LogEvent } from "../types";
import { getAllInstances } from "../services/tauri-bridge";

const LOG_MAX_SIZE = 1000;
const LOG_TRIM_SIZE = 500;

let instances = $state<InstanceMap>({});

// Per-instance logs: name → log entries
let logs = $state<Record<string, { stream: string; line: string; ts: number }[]>>({});

// ── Per-instance perf stats ───────────────────────────────────────────────────

export interface InstancePerfStats {
  evalTps: number | null;       // 最近一次推理输出速度 (tokens/s)
  promptTps: number | null;     // 最近一次 prompt 处理速度 (tokens/s)
  totalPromptTokens: number;    // 累计输入 token 数
  totalEvalTokens: number;      // 累计输出 token 数
}

let perfStats = $state<Record<string, InstancePerfStats>>({});

// 匹配格式（新版 slot print_timing 和旧版 llama_print_timings 均兼容）：
//   新版：prompt eval time =    7296.97 ms / 25420 tokens (..., 3483.64 tokens per second)
//   新版：       eval time =   12075.89 ms /   769 tokens (...,   63.68 tokens per second)
//   旧版：llama_print_timings:        eval time = xxx ms / 512 tokens (..., 63.0 tokens per second)
//   旧版：llama_print_timings: prompt eval time = xxx ms / 128 tokens (..., 244.6 tokens per second)
const RE_EVAL = /(?<!prompt )eval time\s*=\s*[\d.]+\s*ms\s*\/\s*(\d+)\s*tokens[^,]+,\s*([\d.]+)\s+tokens per second/;
const RE_PROMPT = /prompt eval time\s*=\s*[\d.]+\s*ms\s*\/\s*(\d+)\s*tokens[^,]+,\s*([\d.]+)\s+tokens per second/;

function ensurePerf(name: string): InstancePerfStats {
  if (!perfStats[name]) {
    perfStats[name] = { evalTps: null, promptTps: null, totalPromptTokens: 0, totalEvalTokens: 0 };
  }
  return perfStats[name];
}

function parsePerfLine(instanceName: string, line: string) {
  const em = RE_EVAL.exec(line);
  if (em) {
    const p = ensurePerf(instanceName);
    p.totalEvalTokens += parseInt(em[1], 10);
    p.evalTps = parseFloat(em[2]);
    return;
  }
  const pm = RE_PROMPT.exec(line);
  if (pm) {
    const p = ensurePerf(instanceName);
    p.totalPromptTokens += parseInt(pm[1], 10);
    p.promptTps = parseFloat(pm[2]);
  }
}

let _initialized = false;
let _unlisteners: UnlistenFn[] = [];

export function getInstanceStore() {
  if (!_initialized) {
    _initialized = true;

    listen<InstanceMap>("llama://instances", (event) => {
      instances = event.payload;
      // 清理已不存在实例的统计（实例停止后移除）
      for (const name of Object.keys(perfStats)) {
        if (!event.payload[name]) delete perfStats[name];
      }
    }).then((fn) => _unlisteners.push(fn));

    listen<LogEvent>("llama://log", (event) => {
      const { instance, stream, line } = event.payload;
      if (!logs[instance]) logs[instance] = [];
      const bucket = logs[instance];
      if (bucket.length >= LOG_MAX_SIZE) logs[instance] = bucket.slice(LOG_TRIM_SIZE);
      logs[instance].push({ stream, line, ts: Date.now() });
      parsePerfLine(instance, line);
    }).then((fn) => _unlisteners.push(fn));

    // Hydrate from backend on first load
    getAllInstances().then((map) => { instances = map; }).catch(() => {});
  }

  return {
    get instances() { return instances; },
    get logs() { return logs; },
    get perfStats() { return perfStats; },

    // 向后兼容：取最近有数据的实例的 evalTps
    get tokensPerSec(): number | null {
      for (const s of Object.values(perfStats)) {
        if (s.evalTps != null) return s.evalTps;
      }
      return null;
    },
    get promptTps(): number | null {
      for (const s of Object.values(perfStats)) {
        if (s.promptTps != null) return s.promptTps;
      }
      return null;
    },

    getInstancePerf(name: string): InstancePerfStats | null {
      return perfStats[name] ?? null;
    },

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
