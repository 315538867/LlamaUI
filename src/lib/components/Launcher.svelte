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
      if (p.ctx_size != null) ctxSize = p.ctx_size;
      if (p.threads != null) threads = p.threads;
      if (p.port != null) port = p.port;
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

<div class="flex h-full flex-col gap-3 p-4">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold">启动器</h2>
    <div class="flex items-center gap-2">
      {#if process.isRunning || process.isStarting}
        <button
          class="rounded-lg bg-[var(--danger)] px-4 py-1.5 text-sm font-medium text-white transition-colors hover:opacity-90"
          onclick={handleStop}
        >
          停止
        </button>
      {:else}
        <button
          class="rounded-lg bg-[var(--accent)] px-4 py-1.5 text-sm font-medium text-white transition-colors hover:bg-[var(--accent-hover)] disabled:opacity-50"
          onclick={handleStart}
          disabled={!selectedModel || launching}
        >
          {launching ? "启动中..." : "启动"}
        </button>
      {/if}
    </div>
  </div>

  {#if launchError}
    <div class="rounded-lg bg-[var(--danger)]/10 px-3 py-2 text-xs text-[var(--danger)]">
      {launchError}
    </div>
  {/if}

  <!-- Config Panel -->
  <div class="grid grid-cols-2 gap-3 rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] p-3 lg:grid-cols-4">
    <!-- Model Selection -->
    <div class="col-span-2">
      <label class="mb-1 block text-xs text-[var(--text-muted)]">模型</label>
      <select
        bind:value={selectedModel}
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      >
        <option value="">选择模型...</option>
        {#each modelStore.models as model}
          <option value={model.path}>
            {model.name} ({model.size_display}{model.quantization ? ` · ${model.quantization}` : ""})
          </option>
        {/each}
      </select>
    </div>

    <!-- Mode -->
    <div>
      <label class="mb-1 block text-xs text-[var(--text-muted)]">模式</label>
      <select
        bind:value={mode}
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      >
        <option value="server">Server (HTTP API)</option>
        <option value="cli">CLI (命令行)</option>
      </select>
    </div>

    <!-- GPU Layers -->
    <div>
      <label class="mb-1 block text-xs text-[var(--text-muted)]">GPU 层数</label>
      <input
        type="number"
        bind:value={gpuLayers}
        min="-1"
        max="999"
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      />
    </div>

    <!-- Context Size -->
    <div>
      <label class="mb-1 block text-xs text-[var(--text-muted)]">上下文长度</label>
      <input
        type="number"
        bind:value={ctxSize}
        min="128"
        step="256"
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      />
    </div>

    <!-- Threads -->
    <div>
      <label class="mb-1 block text-xs text-[var(--text-muted)]">线程数 (0=自动)</label>
      <input
        type="number"
        bind:value={threads}
        min="0"
        max="256"
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      />
    </div>

    {#if mode === "server"}
      <!-- Port -->
      <div>
        <label class="mb-1 block text-xs text-[var(--text-muted)]">端口</label>
        <input
          type="number"
          bind:value={port}
          min="1024"
          max="65535"
          class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
        />
      </div>

      <!-- Toggles -->
      <div class="col-span-2 flex items-end gap-4 lg:col-span-1">
        <label class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
          <input type="checkbox" bind:checked={flashAttn} class="h-3.5 w-3.5" />
          Flash Attn
        </label>
        <label class="flex items-center gap-1.5 text-xs text-[var(--text-secondary)]">
          <input type="checkbox" bind:checked={contBatching} class="h-3.5 w-3.5" />
          连续批处理
        </label>
      </div>
    {/if}

    <!-- Extra Args -->
    <div class="col-span-2 lg:col-span-4">
      <label class="mb-1 block text-xs text-[var(--text-muted)]">额外参数</label>
      <input
        type="text"
        bind:value={extraArgs}
        placeholder="--no-mmap --verbose ..."
        class="w-full rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
      />
    </div>
  </div>

  <!-- Log Terminal -->
  <div class="min-h-0 flex-1">
    <LogTerminal />
  </div>
</div>
