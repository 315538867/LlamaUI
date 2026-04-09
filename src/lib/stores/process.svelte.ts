import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { InstanceMap, InstanceInfo, PerfEvent } from "../types";
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

interface LogEntry { stream: string; line: string; }
interface LogBatchPayload { instance: string; entries: LogEntry[]; }

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

    listen<LogBatchPayload>("llama://log/batch", (event) => {
      const { instance, entries } = event.payload;
      const now = Date.now();
      const incoming = entries.map((e) => ({ stream: e.stream, line: e.line, ts: now }));

      // One reactive assignment — concat the whole batch at once
      const bucket = logs[instance] ?? [];
      const merged = bucket.concat(incoming);
      logs[instance] = merged.length >= LOG_MAX_SIZE
        ? merged.slice(merged.length - LOG_TRIM_SIZE)
        : merged;
    }).then((fn) => _unlisteners.push(fn));

    // Structured perf telemetry from Rust (replaces frontend regex parsing)
    listen<PerfEvent>("llama://perf", (event) => {
      const { instance, eval_tps, prompt_tps, eval_tokens, prompt_tokens } = event.payload;
      const prev = perfStats[instance] ?? {
        evalTps: null, promptTps: null, totalPromptTokens: 0, totalEvalTokens: 0,
      };
      perfStats[instance] = {
        evalTps: eval_tps ?? prev.evalTps,
        promptTps: prompt_tps ?? prev.promptTps,
        totalEvalTokens: prev.totalEvalTokens + (eval_tokens ?? 0),
        totalPromptTokens: prev.totalPromptTokens + (prompt_tokens ?? 0),
      };
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
