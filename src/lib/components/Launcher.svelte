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
  let flashAttn = $state(true);
  let contBatching = $state(true);
  let extraArgs = $state("");
  let launching = $state(false);
  let launchError = $state<string | null>(null);
  let activePreset = $state<string | null>(null);

  // RTX 5080 16GB 预设（按模型档位）
  const GPU_PRESETS = [
    { label: "9B Q4",  gpu_layers: 99, ctx_size: 32768, flash_attn: true, match: /9b/i },
    { label: "9B Q8",  gpu_layers: 99, ctx_size: 16384, flash_attn: true, match: /9b/i },
    { label: "14B Q4", gpu_layers: 99, ctx_size: 16384, flash_attn: true, match: /14b/i },
    { label: "14B Q8", gpu_layers: 70, ctx_size: 8192,  flash_attn: true, match: /14b/i },
    { label: "27B Q4", gpu_layers: 30, ctx_size: 8192,  flash_attn: true, match: /27b/i },
  ] as const;

  // 根据选中模型文件名推断匹配预设
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

  // Load models on mount
  $effect(() => {
    if (configStore.loaded && modelStore.models.length === 0) {
      modelStore.refresh();
    }
  });

  // Apply default params from config
  $effect(() => {
    if (configStore.loaded) {
      const p = configStore.config.default_params;
      if (p.gpu_layers != null) gpuLayers = p.gpu_layers;
      if (p.ctx_size   != null) ctxSize   = p.ctx_size;
      if (p.threads    != null) threads   = p.threads;
      if (p.port       != null) port      = p.port;
      if (p.flash_attn != null) flashAttn = p.flash_attn;
      if (p.cont_batching != null) contBatching = p.cont_batching;
    }
  });

  async function handleStart() {
    if (!selectedModel) return;
    launching = true;
    launchError = null;
    try {
      const config: LaunchConfig = {
        model_path: selectedModel,
        mode,
        gpu_layers: gpuLayers,
        ctx_size: ctxSize,
        threads: threads > 0 ? threads : undefined,
        port: mode === "server" ? port : undefined,
        host: mode === "server" ? "127.0.0.1" : undefined,
        flash_attn: mode === "server" ? flashAttn : undefined,
        cont_batching: mode === "server" ? contBatching : undefined,
        extra_args: extraArgs || undefined,
      };
      await startLlama(config);
    } catch (e) {
      launchError = String(e);
    } finally {
      launching = false;
    }
  }

  async function handleStop() {
    try {
      await stopLlama();
    } catch (e) {
      console.error("Stop failed:", e);
    }
  }
</script>

<div class="flex h-full flex-col" style="background:var(--bg-base);">

  <!-- ── 顶部栏 ── -->
  <div
    class="flex shrink-0 items-center justify-between border-b px-4 py-3"
    style="border-color:var(--border-subtle); background:var(--bg-surface);"
  >
    <div>
      <h2 class="text-sm font-semibold" style="color:var(--text-base);">启动器</h2>
      <p class="text-xs" style="color:var(--text-muted);">配置并启动 llama.cpp 推理服务</p>
    </div>

    <div class="flex items-center gap-2">
      {#if process.isRunning || process.isStarting}
        <button
          onclick={handleStop}
          class="rounded-md px-4 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-85"
          style="background:var(--danger);"
        >
          停止运行
        </button>
      {:else}
        <button
          onclick={handleStart}
          disabled={!selectedModel || launching}
          class="rounded-md px-4 py-1.5 text-xs font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-40"
          style="background:var(--accent);"
        >
          {launching ? "启动中..." : "▶ 启动"}
        </button>
      {/if}
    </div>
  </div>

  <!-- ── 错误提示 ── -->
  {#if launchError}
    <div
      class="mx-4 mt-3 rounded-md px-3 py-2 text-xs"
      style="background:var(--danger-subtle); color:var(--danger); border:1px solid rgba(239,68,68,0.25);"
    >
      {launchError}
    </div>
  {/if}

  <!-- ── 配置区 ── -->
  <div class="shrink-0 px-4 pt-3 pb-2 space-y-3">

    <!-- GPU 快速预设 -->
    <div
      class="rounded-lg border p-3"
      style="background:var(--bg-surface); border-color:var(--border-subtle);"
    >
      <div class="mb-2 flex items-center justify-between">
        <span class="text-xs font-medium" style="color:var(--text-secondary);">
          RTX 5080 快速预设
        </span>
        {#if suggestedPresets.length > 0}
          <span class="text-[10px] rounded px-1.5 py-0.5" style="background:var(--accent-subtle); color:var(--accent);">
            已匹配 {suggestedPresets.length} 个推荐
          </span>
        {/if}
      </div>
      <div class="flex flex-wrap gap-1.5">
        {#each GPU_PRESETS as preset}
          {@const suggested = suggestedPresets.includes(preset)}
          <button
            onclick={() => applyPreset(preset)}
            class="rounded px-2.5 py-1 text-xs transition-all"
            style={activePreset === preset.label
              ? "background:var(--accent); color:#fff; font-weight:600;"
              : suggested
                ? "background:var(--accent-subtle); color:var(--accent); border:1px solid rgba(59,130,246,0.3);"
                : "background:var(--bg-elevated); color:var(--text-secondary); border:1px solid var(--border-subtle);"}
          >
            {preset.label}
            <span class="ml-1 opacity-60">{(preset.ctx_size / 1024).toFixed(0)}K</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- 参数配置 -->
    <div
      class="rounded-lg border p-3"
      style="background:var(--bg-surface); border-color:var(--border-subtle);"
    >
      <div class="grid grid-cols-2 gap-x-3 gap-y-2.5 lg:grid-cols-4">

        <!-- 模型 -->
        <div class="col-span-2">
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">模型文件
            <select
              bind:value={selectedModel}
              onchange={() => (activePreset = null)}
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            >
              <option value="">选择模型...</option>
              {#each modelStore.models as model}
                <option value={model.path}>
                  {model.name}{model.quantization ? ` · ${model.quantization}` : ""} ({model.size_display})
                </option>
              {/each}
            </select>
          </label>
        </div>

        <!-- 模式 -->
        <div>
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">运行模式
            <select
              bind:value={mode}
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            >
              <option value="server">Server</option>
              <option value="cli">CLI</option>
            </select>
          </label>
        </div>

        <!-- GPU 层数 -->
        <div>
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">GPU 层数
            <input
              type="number" bind:value={gpuLayers} min="-1" max="999"
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            />
          </label>
        </div>

        <!-- 上下文 -->
        <div>
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">上下文长度
            <input
              type="number" bind:value={ctxSize} min="128" step="512"
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            />
          </label>
        </div>

        <!-- 线程 -->
        <div>
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">线程 (0=自动)
            <input
              type="number" bind:value={threads} min="0" max="256"
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            />
          </label>
        </div>

        {#if mode === "server"}
          <!-- 端口 -->
          <div>
            <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">端口
              <input
                type="number" bind:value={port} min="1024" max="65535"
                class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
                style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
              />
            </label>
          </div>

          <!-- 开关 -->
          <div class="col-span-2 flex items-end gap-4 pb-0.5 lg:col-span-2">
            <label class="flex items-center gap-1.5 text-xs cursor-pointer" style="color:var(--text-secondary);">
              <input type="checkbox" bind:checked={flashAttn} class="h-3.5 w-3.5 accent-blue-500" />
              Flash Attention
            </label>
            <label class="flex items-center gap-1.5 text-xs cursor-pointer" style="color:var(--text-secondary);">
              <input type="checkbox" bind:checked={contBatching} class="h-3.5 w-3.5 accent-blue-500" />
              连续批处理
            </label>
          </div>
        {/if}

        <!-- 额外参数 -->
        <div class="col-span-2 lg:col-span-4">
          <label class="block text-[11px] font-medium mb-1" style="color:var(--text-muted);">额外参数
            <input
              type="text" bind:value={extraArgs} placeholder="--no-mmap --verbose ..."
              class="mt-1 w-full rounded-md border px-2.5 py-1.5 text-xs"
              style="background:var(--bg-elevated); border-color:var(--border-subtle); color:var(--text-base);"
            />
          </label>
        </div>
      </div>
    </div>
  </div>

  <!-- ── 日志终端 ── -->
  <div class="min-h-0 flex-1 px-4 pb-3">
    <LogTerminal />
  </div>
</div>
