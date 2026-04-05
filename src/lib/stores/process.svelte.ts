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

// 最近一次生成速度（tokens/s），从 eval time 行解析
let tokensPerSec = $state<number | null>(null);
// 最近一次 prompt eval 速度（prefill t/s）
let promptTps = $state<number | null>(null);

// eval time 行格式（两种 llama.cpp 版本）：
//   llama_print_timings:        eval time = 5678.90 ms /   99 runs   (  57.36 ms per token,   17.43 tokens per second)
//   llama_perf_context_print:   eval time = 5678.90 ms /   99 runs   (  57.36 ms per token,   17.43 tokens per second)
const RE_EVAL = /(?:llama_print_timings|llama_perf_context_print):\s+eval time[^,]+,\s*([\d.]+)\s+tokens per second/;
const RE_PROMPT = /(?:llama_print_timings|llama_perf_context_print):\s+prompt eval time[^,]+,\s*([\d.]+)\s+tokens per second/;

function parsePerfLine(line: string) {
  const em = RE_EVAL.exec(line);
  if (em) { tokensPerSec = parseFloat(em[1]); return; }
  const pm = RE_PROMPT.exec(line);
  if (pm) { promptTps = parseFloat(pm[1]); }
}

let _initialized = false;
let _unlisteners: UnlistenFn[] = [];

export function getProcessStore() {
  if (!_initialized) {
    _initialized = true;

    listen<ProcessInfo>("llama://status-change", (event) => {
      processInfo = event.payload;
      // 服务停止时清除速度
      if (event.payload.status === "stopped" || event.payload.status === "error") {
        tokensPerSec = null;
        promptTps = null;
      }
    }).then((fn) => _unlisteners.push(fn));

    listen<LogEvent>("llama://log", (event) => {
      if (logs.length >= 1000) logs = logs.slice(500);
      logs.push({ ...event.payload, ts: Date.now() });
      parsePerfLine(event.payload.line);
    }).then((fn) => _unlisteners.push(fn));

    getLlamaStatus().then((info) => { processInfo = info; });
  }

  return {
    get info()        { return processInfo; },
    get logs()        { return logs; },
    get isRunning()   { return processInfo.status === "running"; },
    get isStarting()  { return processInfo.status === "starting"; },
    get tokensPerSec(){ return tokensPerSec; },
    get promptTps()   { return promptTps; },
    clearLogs() { logs = []; },
    destroy() {
      if (!_initialized) return;
      for (const fn of _unlisteners) fn();
      _unlisteners = [];
      _initialized = false;
    },
  };
}
