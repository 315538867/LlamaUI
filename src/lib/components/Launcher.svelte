<script lang="ts">
  import { getProcessStore } from "../stores/process.svelte";
  import { getModelStore } from "../stores/models.svelte";
  import { getConfigStore } from "../stores/config.svelte";
  import { startLlama, stopLlama } from "../services/tauri-bridge";
  import LogTerminal from "./LogTerminal.svelte";
  import type { LaunchMode, LaunchConfig } from "../types";

  const process = getProcessStore();
  const modelStore = getModelStore();
  const configStore = getConfigStore();

  let selectedModel = $state("");
  let mode = $state<LaunchMode>("server");
  let gpuLayers = $state(99);
  let ctxSize = $state(4096);
  let threads = $state(0);
  let port = $state(8080);
  let host = $state("127.0.0.1");
  let flashAttn = $state(true);
  let contBatching = $state(true);
  let batchSize = $state(512);
  let ubatchSize = $state(512);
  let parallel = $state(1);
  let cacheTypeK = $state("f16");
  let seed = $state(-1);
  let mlock = $state(false);
  let noMmap = $state(false);
  let apiKey = $state("");
  let corsAllowOrigins = $state("*");
  let extraArgs = $state("");
  let launching = $state(false);
  let launchError = $state<string | null>(null);
  let activePreset = $state<string | null>(null);
  let showAdvanced = $state(false);

  // RTX 5080 16GB 预设（按模型档位）
  const GPU_PRESETS = [
    { label: "9B Q4",  gpu_layers: 99, ctx_size: 32768, flash_attn: true, match: /9b/i },
    { label: "9B Q8",  gpu_layers: 99, ctx_size: 16384, flash_attn: true, match: /9b/i },
    { label: "14B Q4", gpu_layers: 99, ctx_size: 16384, flash_attn: true, match: /14b/i },
    { label: "14B Q8", gpu_layers: 70, ctx_size: 8192,  flash_attn: true, match: /14b/i },
    { label: "27B Q4", gpu_layers: 30, ctx_size: 8192,  flash_attn: true, match: /27b/i },
  ] as const;

  const suggestedPresets = $derived(
    selectedModel
      ? GPU_PRESETS.filter(p => p.match.test(selectedModel.split(/[/\\]/).pop() ?? ""))
      : []
  );

  function applyPreset(preset: typeof GPU_PRESETS[number]) {
    gpuLayers  = preset.gpu_layers;
    ctxSize    = preset.ctx_size;
    flashAttn  = preset.flash_attn;
    activePreset = preset.label;
  }

  $effect(() => {
    if (configStore.loaded && modelStore.models.length === 0) {
      modelStore.refresh();
    }
  });

  $effect(() => {
    if (configStore.loaded) {
      const p = configStore.config.default_params;
      if (p.gpu_layers   != null) gpuLayers   = p.gpu_layers;
      if (p.ctx_size     != null) ctxSize     = p.ctx_size;
      if (p.threads      != null) threads     = p.threads;
      if (p.port         != null) port        = p.port;
      if (p.host         != null) host        = p.host;
      if (p.flash_attn   != null) flashAttn   = p.flash_attn;
      if (p.cont_batching != null) contBatching = p.cont_batching;
      if (p.batch_size   != null) batchSize   = p.batch_size;
      if (p.ubatch_size  != null) ubatchSize  = p.ubatch_size;
      if (p.parallel     != null) parallel    = p.parallel;
      if (p.cache_type_k != null) cacheTypeK  = p.cache_type_k;
      if (p.seed         != null) seed        = p.seed;
      if (p.mlock        != null) mlock       = p.mlock;
      if (p.no_mmap      != null) noMmap      = p.no_mmap;
      if (p.api_key      != null) apiKey      = p.api_key;
      if (p.cors_allow_origins != null) corsAllowOrigins = p.cors_allow_origins;
    }
  });

  async function handleStart() {
    if (!selectedModel) return;
    launching = true;
    launchError = null;
    try {
      const isServer = mode === "server";
      const config: LaunchConfig = {
        model_path: selectedModel,
        mode,
        gpu_layers: gpuLayers,
        ctx_size:   ctxSize,
        threads:    threads > 0 ? threads : undefined,
        port:       isServer ? port : undefined,
        host:       isServer ? host : undefined,
        flash_attn: isServer ? flashAttn : undefined,
        cont_batching: isServer ? contBatching : undefined,
        batch_size:  isServer ? batchSize : undefined,
        ubatch_size: isServer ? ubatchSize : undefined,
        parallel:    isServer ? parallel : undefined,
        cache_type_k: isServer && cacheTypeK !== "f16" ? cacheTypeK : undefined,
        seed:        seed !== -1 ? seed : undefined,
        mlock:       mlock || undefined,
        no_mmap:     noMmap || undefined,
        api_key:     isServer && apiKey ? apiKey : undefined,
        cors_allow_origins: isServer && corsAllowOrigins ? corsAllowOrigins : undefined,
        extra_args:  extraArgs || undefined,
      };
      await startLlama(config);
    } catch (e) {
      launchError = String(e);
    } finally {
      launching = false;
    }
  }

  async function handleStop() {
    try { await stopLlama(); } catch (e) { console.error("Stop failed:", e); }
  }
</script>

<div class="launcher-root">

  <!-- ── 顶部操作栏 ── -->
  <div class="topbar">
    <div>
      <div class="topbar-title">启动器</div>
      <div class="topbar-sub">配置并运行 llama.cpp 推理服务</div>
    </div>
    <div class="topbar-actions">
      {#if process.isRunning || process.isStarting}
        <button class="btn-stop" onclick={handleStop}>■ 停止</button>
      {:else}
        <button
          class="btn-start"
          onclick={handleStart}
          disabled={!selectedModel || launching}
        >
          {launching ? "启动中..." : "▶ 启动"}
        </button>
      {/if}
    </div>
  </div>

  {#if launchError}
    <div class="error-bar">{launchError}</div>
  {/if}

  <!-- ── 配置面板 ── -->
  <div class="config-panel">

    <!-- 模型选择 -->
    <div class="section">
      <div class="section-title">模型</div>
      <select class="input w-full" bind:value={selectedModel} onchange={() => (activePreset = null)}>
        <option value="">选择模型文件...</option>
        {#each modelStore.models as model}
          <option value={model.path}>
            {model.name}{model.quantization ? ` · ${model.quantization}` : ""} ({model.size_display})
          </option>
        {/each}
      </select>
    </div>

    <!-- GPU 快速预设 -->
    <div class="section">
      <div class="section-header">
        <span class="section-title">RTX 5080 快速预设</span>
        {#if suggestedPresets.length > 0}
          <span class="badge-accent">推荐 {suggestedPresets.length} 个</span>
        {/if}
      </div>
      <div class="preset-row">
        {#each GPU_PRESETS as preset}
          {@const suggested = suggestedPresets.includes(preset)}
          {@const active = activePreset === preset.label}
          <button
            onclick={() => applyPreset(preset)}
            class="preset-btn"
            class:preset-active={active}
            class:preset-suggested={suggested && !active}
          >
            {preset.label}
            <span class="preset-ctx">{(preset.ctx_size / 1024).toFixed(0)}K</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- 基础配置 -->
    <div class="section">
      <div class="section-title">基础配置</div>
      <div class="grid-2">
        <label class="field">
          <span class="field-label">运行模式</span>
          <select class="input" bind:value={mode}>
            <option value="server">Server — HTTP API 服务</option>
            <option value="cli">CLI — 命令行对话</option>
          </select>
        </label>
        <label class="field">
          <span class="field-label">GPU 层数 <span class="hint">-ngl，越大显存占用越高</span></span>
          <input class="input" type="number" bind:value={gpuLayers} min="-1" max="999" />
        </label>
        <label class="field">
          <span class="field-label">上下文长度 <span class="hint">-c，Token 数量，影响显存</span></span>
          <input class="input" type="number" bind:value={ctxSize} min="128" step="512" />
        </label>
        <label class="field">
          <span class="field-label">CPU 线程 <span class="hint">-t，0 = 自动检测</span></span>
          <input class="input" type="number" bind:value={threads} min="0" max="256" />
        </label>
      </div>
    </div>

    {#if mode === "server"}
    <!-- 服务配置 -->
    <div class="section">
      <div class="section-title">服务配置</div>
      <div class="grid-2">
        <label class="field">
          <span class="field-label">监听地址 <span class="hint">--host，0.0.0.0 允许外部访问</span></span>
          <select class="input" bind:value={host}>
            <option value="127.0.0.1">127.0.0.1 — 仅本机</option>
            <option value="0.0.0.0">0.0.0.0 — 开放网络访问</option>
          </select>
        </label>
        <label class="field">
          <span class="field-label">端口 <span class="hint">--port，默认 8080</span></span>
          <input class="input" type="number" bind:value={port} min="1024" max="65535" />
        </label>
      </div>
      <div class="grid-2" style="margin-top:8px;">
        <label class="field">
          <span class="field-label">API Key <span class="hint">--api-key，留空则无需鉴权</span></span>
          <input class="input font-mono" type="text" bind:value={apiKey} placeholder="sk-..." autocomplete="off" />
        </label>
        <label class="field">
          <span class="field-label">CORS 允许来源 <span class="hint">--cors-allow-origins，* = 所有来源</span></span>
          <input class="input font-mono" type="text" bind:value={corsAllowOrigins} placeholder="* 或 http://localhost:3000" />
        </label>
      </div>
      <div class="switch-row">
        <label class="switch-item">
          <input type="checkbox" bind:checked={flashAttn} />
          <span>Flash Attention <span class="hint">--flash-attn，加速注意力计算（需 GPU 支持）</span></span>
        </label>
        <label class="switch-item">
          <input type="checkbox" bind:checked={contBatching} />
          <span>连续批处理 <span class="hint">--cont-batching，提升多并发吞吐量</span></span>
        </label>
      </div>
    </div>
    {/if}

    <!-- 高级配置（可折叠） -->
    <div class="section">
      <button class="section-toggle" onclick={() => showAdvanced = !showAdvanced}>
        <span class="section-title">高级配置</span>
        <span class="toggle-icon">{showAdvanced ? "▾" : "▸"}</span>
      </button>

      {#if showAdvanced}
      <div class="grid-2" style="margin-top:0.5rem;">
        <label class="field">
          <span class="field-label">批处理大小 <span class="hint">-b，提示词处理批次，默认 512</span></span>
          <input class="input" type="number" bind:value={batchSize} min="1" max="8192" />
        </label>
        <label class="field">
          <span class="field-label">解码批次 <span class="hint">-ub，推理批次大小，默认 512</span></span>
          <input class="input" type="number" bind:value={ubatchSize} min="1" max="8192" />
        </label>
        <label class="field">
          <span class="field-label">并行槽位 <span class="hint">--parallel，同时处理请求数</span></span>
          <input class="input" type="number" bind:value={parallel} min="1" max="64" />
        </label>
        <label class="field">
          <span class="field-label">KV 缓存类型 <span class="hint">--cache-type-k，量化可省显存</span></span>
          <select class="input" bind:value={cacheTypeK}>
            <option value="f16">f16 — 默认精度</option>
            <option value="q8_0">q8_0 — 8位量化</option>
            <option value="q4_0">q4_0 — 4位量化（省显存）</option>
          </select>
        </label>
        <label class="field">
          <span class="field-label">随机种子 <span class="hint">--seed，-1 = 随机</span></span>
          <input class="input" type="number" bind:value={seed} min="-1" />
        </label>
      </div>
      <div class="switch-row">
        <label class="switch-item">
          <input type="checkbox" bind:checked={mlock} />
          <span>锁定内存 <span class="hint">--mlock，防止模型被换出到磁盘</span></span>
        </label>
        <label class="switch-item">
          <input type="checkbox" bind:checked={noMmap} />
          <span>禁用 mmap <span class="hint">--no-mmap，完整加载到内存而非映射</span></span>
        </label>
      </div>
      {/if}
    </div>

    <!-- 额外参数 -->
    <div class="section">
      <label class="field">
        <span class="field-label">额外参数 <span class="hint">直接追加到命令行，如 --verbose --log-file /tmp/llama.log</span></span>
        <input
          class="input font-mono"
          type="text"
          bind:value={extraArgs}
          placeholder="--verbose ..."
        />
      </label>
    </div>

  </div>

  <!-- ── 日志终端 ── -->
  <div class="log-area">
    <LogTerminal />
  </div>
</div>

<style>
.launcher-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-base);
  overflow: hidden;
}

/* ─ Topbar ─ */
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}
.topbar-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-base);
  line-height: 1.2;
}
.topbar-sub {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 1px;
}
.topbar-actions { display: flex; gap: 6px; }
.btn-start {
  padding: 5px 14px;
  font-size: 12px;
  font-weight: 600;
  color: #fff;
  background: var(--accent);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.btn-start:hover { opacity: 0.88; }
.btn-start:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-stop {
  padding: 5px 14px;
  font-size: 12px;
  font-weight: 500;
  color: #fff;
  background: var(--danger);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.15s;
}
.btn-stop:hover { opacity: 0.85; }

/* ─ Error ─ */
.error-bar {
  margin: 8px 16px;
  padding: 8px 12px;
  font-size: 11px;
  color: var(--danger);
  background: rgba(239,68,68,0.08);
  border: 1px solid rgba(239,68,68,0.2);
  border-radius: 4px;
  flex-shrink: 0;
}

/* ─ Config panel ─ */
.config-panel {
  flex-shrink: 0;
  overflow-y: auto;
  padding: 10px 16px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 62%;
}

/* ─ Section ─ */
.section {
  padding: 8px 0 10px;
  border-bottom: 1px solid var(--border-subtle);
}
.section:last-child { border-bottom: none; }
.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 6px;
  display: block;
}
.section-header .section-title { margin-bottom: 0; }

.section-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  margin-bottom: 0;
}
.section-toggle .section-title { margin-bottom: 0; }
.toggle-icon {
  font-size: 11px;
  color: var(--text-muted);
}

/* ─ Badge ─ */
.badge-accent {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--accent-subtle);
  color: var(--accent);
}

/* ─ Presets ─ */
.preset-row { display: flex; flex-wrap: wrap; gap: 5px; }
.preset-btn {
  padding: 3px 9px;
  font-size: 11px;
  border-radius: 3px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-elevated);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.12s;
  display: flex;
  align-items: center;
  gap: 4px;
}
.preset-btn:hover { border-color: var(--border); color: var(--text-base); }
.preset-active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}
.preset-suggested {
  background: var(--accent-subtle);
  border-color: rgba(59,130,246,0.35);
  color: var(--accent);
}
.preset-ctx { opacity: 0.55; font-size: 10px; }

/* ─ Grid ─ */
.grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 12px;
}

/* ─ Field ─ */
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.field-label {
  font-size: 11px;
  color: var(--text-secondary);
  font-weight: 500;
}
.hint {
  color: var(--text-muted);
  font-weight: 400;
  font-size: 10px;
  margin-left: 3px;
}

/* ─ Input ─ */
.input {
  height: 26px;
  padding: 0 8px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  width: 100%;
  outline: none;
  transition: border-color 0.12s;
}
.input:focus { border-color: var(--accent); }
.input.w-full { width: 100%; }
.input.font-mono { font-family: monospace; }

/* ─ Switch row ─ */
.switch-row {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
.switch-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}
.switch-item input[type="checkbox"] {
  width: 13px;
  height: 13px;
  accent-color: var(--accent);
  flex-shrink: 0;
}

/* ─ Log area ─ */
.log-area {
  flex: 1;
  min-height: 0;
  padding: 0 16px 12px;
}
</style>
