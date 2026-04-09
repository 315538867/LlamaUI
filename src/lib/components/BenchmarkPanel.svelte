<script lang="ts">
  import { onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { BenchmarkLine, BenchmarkDone } from "../types";
  import { runBenchmark } from "../services/tauri-bridge";
  import { getConfigStore } from "../stores/config.svelte";

  interface Props {
    instanceName: string;
    modelPath: string;
  }

  let { instanceName, modelPath }: Props = $props();

  const configStore = getConfigStore();

  // ── Params ────────────────────────────────────────────────────────────────

  let threads = $state<string>("");           // "" = 不传（llama-bench 使用默认）
  let promptTokens = $state<number>(512);
  let genTokens = $state<number>(128);

  // ── Runtime state ─────────────────────────────────────────────────────────

  let running = $state(false);
  let exitCode = $state<number | null>(null);
  let lines = $state<string[]>([]);
  let logEl: HTMLDivElement;
  let rafId: number | undefined;
  let unlisteners: UnlistenFn[] = [];

  // Auto-scroll when lines change
  $effect(() => {
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    lines.length;
    if (logEl) {
      if (rafId !== undefined) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        logEl.scrollTop = logEl.scrollHeight;
      });
    }
  });

  // ── Helpers ───────────────────────────────────────────────────────────────

  function clearListeners() {
    for (const fn of unlisteners) fn();
    unlisteners = [];
  }

  async function startBench() {
    const llamaDir = configStore.config.llama_dir;
    if (!llamaDir) return;

    clearListeners();
    lines = [];
    exitCode = null;
    running = true;

    // Register event listeners BEFORE invoking so we don't miss early lines
    const unlinesLine = await listen<BenchmarkLine>("llama://bench/line", (ev) => {
      if (ev.payload.instance !== instanceName) return;
      lines = [...lines, ev.payload.line];
    });
    const unlistenDone = await listen<BenchmarkDone>("llama://bench/done", (ev) => {
      if (ev.payload.instance !== instanceName) return;
      exitCode = ev.payload.exit_code;
      running = false;
      clearListeners();
    });
    unlisteners = [unlinesLine, unlistenDone];

    try {
      await runBenchmark(instanceName, modelPath, llamaDir, {
        threads: threads.trim() !== "" ? parseInt(threads) : undefined,
        prompt_tokens: promptTokens,
        gen_tokens: genTokens,
      });
    } catch (e) {
      lines = [...lines, `错误: ${String(e)}`];
      running = false;
      clearListeners();
    }
  }

  onDestroy(() => {
    clearListeners();
    if (rafId !== undefined) cancelAnimationFrame(rafId);
  });

  const llamaDir = $derived(configStore.config.llama_dir ?? null);
  const canRun = $derived(!running && !!llamaDir && !!modelPath);
</script>

<div class="bench-root">
  <!-- ── Params bar ─────────────────────────────────────────────────────── -->
  <div class="params-bar">
    <label class="param-group">
      <span class="param-label">Prompt tokens</span>
      <input class="param-input w-num" type="number" min="1"
        bind:value={promptTokens} disabled={running} />
    </label>
    <label class="param-group">
      <span class="param-label">生成数量</span>
      <input class="param-input w-num" type="number" min="1"
        bind:value={genTokens} disabled={running} />
    </label>
    <label class="param-group">
      <span class="param-label">线程数</span>
      <input class="param-input w-num" type="number" min="1"
        bind:value={threads}
        placeholder="默认"
        disabled={running} />
    </label>

    <div class="param-spacer"></div>

    {#if !llamaDir}
      <span class="warn-hint">⚠ 未配置 llama.cpp 路径</span>
    {/if}

    <button class="run-btn" disabled={!canRun} onclick={startBench}>
      {running ? "运行中..." : "运行 Benchmark"}
    </button>

    {#if lines.length > 0 && !running}
      <button class="clear-btn" onclick={() => { lines = []; exitCode = null; }}>清空</button>
    {/if}
  </div>

  <!-- ── Status hint ────────────────────────────────────────────────────── -->
  {#if exitCode !== null}
    <div class="status-bar" class:ok={exitCode === 0} class:fail={exitCode !== 0}>
      {exitCode === 0 ? "Benchmark 完成" : `进程退出码: ${exitCode}`}
    </div>
  {/if}

  <!-- ── Output terminal ────────────────────────────────────────────────── -->
  <div class="terminal">
    <div class="log-scroll" bind:this={logEl}>
      {#if lines.length === 0}
        <p class="empty">
          {running ? "等待输出..." : "点击「运行 Benchmark」开始测试"}
        </p>
      {:else}
        {#each lines as line}
          <div class="log-line">{line}</div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
.bench-root {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 12px;
  gap: 8px;
}

/* ── Params bar ──────────────────────────────────────────────────────────── */

.params-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  flex-wrap: wrap;
}

.param-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.param-label {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
}

.param-input {
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  font-size: 12px;
  padding: 3px 7px;
  outline: none;
  transition: border-color 0.12s;
}
.param-input:focus { border-color: var(--accent); }
.param-input:disabled { opacity: 0.45; }
.w-num { width: 70px; }

.param-spacer { flex: 1; }

.warn-hint {
  font-size: 11px;
  color: var(--warning);
}

.run-btn {
  font-size: 11px;
  font-weight: 500;
  height: 26px;
  padding: 0 14px;
  border-radius: 4px;
  border: none;
  background: var(--accent);
  color: #fff;
  cursor: pointer;
  transition: opacity 0.12s;
}
.run-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.run-btn:not(:disabled):hover { opacity: 0.85; }

.clear-btn {
  font-size: 10px;
  padding: 2px 8px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
}
.clear-btn:hover { background: var(--bg-hover); }

/* ── Status bar ──────────────────────────────────────────────────────────── */

.status-bar {
  font-size: 11px;
  padding: 4px 10px;
  border-radius: 4px;
  flex-shrink: 0;
}
.status-bar.ok   { background: rgba(16,185,129,0.1); color: var(--success, #10b981); }
.status-bar.fail { background: rgba(239,68,68,0.08); color: var(--danger); }

/* ── Terminal ────────────────────────────────────────────────────────────── */

.terminal {
  background: #0d0d0f;
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.log-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  font-family: monospace;
  font-size: 11px;
  line-height: 1.6;
}

.log-line {
  color: #9ca3af;
  word-break: break-all;
  white-space: pre-wrap;
}

.empty {
  color: var(--text-muted);
  font-size: 11px;
  margin: 0;
}
</style>
