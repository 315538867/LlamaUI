<script lang="ts">
  import { getInstanceStore } from "../stores/process.svelte";
  import { getConfigStore } from "../stores/config.svelte";
  import { getProxyStore } from "../stores/proxy.svelte";
  import {
    startInstance,
    stopInstance,
    deleteInstanceConfig,
    scanModels,
  } from "../services/tauri-bridge";
  import type { InstanceConfig, InstanceInfo, ModelInfo, LaunchParams } from "../types";
  import { logger } from "../utils/logger";
  import LogTerminal from "./LogTerminal.svelte";
  import ProxyConfigPanel from "./ProxyConfigPanel.svelte";
  import InstanceEditForm from "./InstanceEditForm.svelte";

  const instanceStore = getInstanceStore();
  const configStore = getConfigStore();
  const proxyStore = getProxyStore();

  // ── State ─────────────────────────────────────────────────────────────────

  let selectedName = $state<string | null>(null);
  let isCreating = $state(false);
  let createStep = $state<"model" | "config">("model");
  let showProxyPanel = $state(false);
  let activeTab = $state<"config" | "logs">("config");
  let availableModels = $state<ModelInfo[]>([]);
  let scanning = $state(false);
  let saving = $state(false);
  let actionErr = $state("");
  let actionErrTimer: ReturnType<typeof setTimeout> | undefined;
  let paramsCollapsed = $state(false);

  // Edit form state (lifted here, passed down to InstanceEditForm)
  let editName = $state("");
  let editModelPath = $state("");
  let editMode = $state<"server" | "cli">("server");
  let editParams = $state<LaunchParams>(defaultParams());

  function defaultParams(): LaunchParams {
    return {
      gpu_layers: 99,
      ctx_size: 4096,
      threads: null,
      flash_attn: true,
      cont_batching: true,
      batch_size: null,
      ubatch_size: null,
      parallel: null,
      cache_type_k: null,
      cache_type_v: null,
      no_kv_offload: null,
      seed: null,
      mlock: null,
      no_mmap: null,
      system_prompt: null,
      extra_args: null,
      no_context_shift: null,
      keep: null,
    };
  }

  // ── Derived ───────────────────────────────────────────────────────────────

  const savedInstances = $derived(configStore.config.instances ?? []);

  const selectedInfo = $derived<InstanceInfo | null>(
    selectedName ? (instanceStore.instances[selectedName] ?? null) : null
  );

  const selectedConfig = $derived<InstanceConfig | null>(
    selectedName
      ? (savedInstances.find((i) => i.name === selectedName) ?? null)
      : null
  );

  const isRunning = $derived(selectedInfo?.status === "running");
  const isStarting = $derived(selectedInfo?.status === "starting");

  const selectedLogs = $derived(
    selectedName ? instanceStore.getInstanceLogs(selectedName) : []
  );

  // ── Helpers ───────────────────────────────────────────────────────────────

  function showErr(e: unknown) {
    clearTimeout(actionErrTimer);
    actionErr = String(e);
    actionErrTimer = setTimeout(() => { actionErr = ""; }, 4000);
  }

  function loadFormFrom(cfg: InstanceConfig) {
    editName = cfg.name;
    editModelPath = cfg.model_path;
    editMode = cfg.mode;
    editParams = { ...cfg.params };
  }

  function selectInstance(name: string) {
    selectedName = name;
    isCreating = false;
    showProxyPanel = false;
    const cfg = savedInstances.find((i) => i.name === name);
    if (cfg) loadFormFrom(cfg);
  }

  function startCreate() {
    isCreating = true;
    createStep = "model";
    selectedName = null;
    showProxyPanel = false;
    editName = "";
    editModelPath = "";
    editMode = "server";
    editParams = defaultParams();
    activeTab = "config";
    if (availableModels.length === 0) handleScan();
  }

  function generateInstanceName(m: ModelInfo): string {
    let base = m.name;
    if (m.quantization) {
      const re = new RegExp(`[-._]${m.quantization}$`, "i");
      base = base.replace(re, "");
    }
    const taken = new Set(savedInstances.map((i) => i.name));
    if (!taken.has(base)) return base;
    let n = 2;
    while (taken.has(`${base}-${n}`)) n++;
    return `${base}-${n}`;
  }

  function pickModel(m: ModelInfo) {
    editModelPath = m.path;
    editName = generateInstanceName(m);
    createStep = "config";
  }

  async function handleScan() {
    scanning = true;
    try {
      const result = await scanModels();
      availableModels = result.models;
    } catch (e) {
      logger.error("scanModels failed:", e);
    } finally {
      scanning = false;
    }
  }

  function buildConfig(): InstanceConfig {
    return {
      name: editName.trim(),
      model_path: editModelPath,
      mode: editMode,
      params: { ...editParams },
    };
  }

  async function handleSaveAndStart() {
    if (!editName.trim()) { showErr("实例名称不能为空"); return; }
    if (!editModelPath) { showErr("请选择模型文件"); return; }
    saving = true;
    actionErr = "";
    try {
      const cfg = buildConfig();
      await startInstance(cfg);
      await configStore.load();
      selectedName = cfg.name;
      isCreating = false;
      activeTab = "logs";
    } catch (e) {
      showErr(e);
    } finally {
      saving = false;
    }
  }

  async function handleStart() {
    if (!selectedConfig) return;
    saving = true;
    actionErr = "";
    try {
      await startInstance({ ...selectedConfig, ...buildConfig() });
      await configStore.load();
      activeTab = "logs";
    } catch (e) { showErr(e); }
    finally { saving = false; }
  }

  async function handleStop() {
    if (!selectedName) return;
    saving = true;
    actionErr = "";
    try {
      await stopInstance(selectedName);
    } catch (e) { showErr(e); }
    finally { saving = false; }
  }

  async function handleDelete() {
    if (!selectedName) return;
    if (!confirm(`删除实例「${selectedName}」？`)) return;
    saving = true;
    actionErr = "";
    try {
      if (isRunning) await stopInstance(selectedName);
      await deleteInstanceConfig(selectedName);
      await configStore.load();
      selectedName = null;
      isCreating = false;
    } catch (e) { showErr(e); }
    finally { saving = false; }
  }

  function statusColor(info: InstanceInfo | undefined): string {
    if (!info) return "var(--text-muted)";
    switch (info.status) {
      case "running":  return "var(--success)";
      case "starting": return "var(--warning)";
      case "error":    return "var(--danger)";
      default:         return "var(--border)";
    }
  }

  function statusLabel(info: InstanceInfo | undefined): string {
    if (!info) return "已停止";
    switch (info.status) {
      case "running":  return "运行中";
      case "starting": return "启动中";
      case "error":    return "错误";
      default:         return "已停止";
    }
  }

  function filename(path: string): string {
    return path.split(/[/\\]/).pop() ?? path;
  }
</script>

<div class="root">
  <!-- ── Left: Instance list ───────────────────────────────────────────────── -->
  <div class="panel-left">
    <div class="panel-header">
      <span class="panel-title">模型实例</span>
      <button class="btn-new" onclick={startCreate}>+ 新建</button>
    </div>

    <div class="instance-list">
      {#if savedInstances.length === 0 && !isCreating}
        <div class="empty-hint">暂无实例，点击「新建」创建</div>
      {/if}

      {#each savedInstances as cfg (cfg.name)}
        {@const info = instanceStore.instances[cfg.name]}
        <button
          class="instance-item"
          class:selected={selectedName === cfg.name && !isCreating}
          onclick={() => selectInstance(cfg.name)}
        >
          <span class="inst-dot" style="background:{statusColor(info)}"></span>
          <div class="inst-meta">
            <span class="inst-name">{cfg.name}</span>
            <span class="inst-model">{filename(cfg.model_path)}</span>
          </div>
          <span class="inst-status" style="color:{statusColor(info)}">{statusLabel(info)}</span>
        </button>
      {/each}

      {#if isCreating}
        <div class="instance-item selected creating">
          <span class="inst-dot" style="background:var(--accent)"></span>
          <div class="inst-meta">
            <span class="inst-name">{editName || "新实例..."}</span>
            <span class="inst-model">未保存</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="panel-footer">
      <button
        class="proxy-log-btn"
        class:active={showProxyPanel}
        onclick={() => { showProxyPanel = !showProxyPanel; selectedName = null; isCreating = false; }}
      >
        <span class="proxy-dot" class:has-logs={proxyStore.logs.length > 0}></span>
        代理
        {#if proxyStore.logs.length > 0}
          <span class="proxy-count">{proxyStore.logs.length}</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- ── Right: Content panel ─────────────────────────────────────────────── -->
  <div class="panel-right">
    {#if showProxyPanel}
      <ProxyConfigPanel />

    {:else if !selectedName && !isCreating}
      <div class="empty-state">
        <div class="empty-icon">⬡</div>
        <div class="empty-msg">选择或创建一个实例</div>
      </div>

    {:else if isCreating && createStep === "model"}
      <div class="model-pick-panel">
        <div class="model-pick-header">
          <span class="model-pick-title">选择模型文件</span>
          <button class="btn-ghost" onclick={handleScan} disabled={scanning}>
            {scanning ? "扫描中..." : "重新扫描"}
          </button>
          <button class="btn-ghost" onclick={() => { isCreating = false; }}>取消</button>
        </div>
        {#if scanning}
          <div class="model-pick-loading">扫描中...</div>
        {:else if availableModels.length === 0}
          <div class="model-pick-empty">
            未找到模型文件，请先在设置中配置模型目录
          </div>
        {:else}
          <div class="model-pick-list">
            {#each availableModels as m}
              <button class="model-pick-item" onclick={() => pickModel(m)}>
                <div class="model-pick-name">{m.name}</div>
                <div class="model-pick-meta">
                  <span class="model-pick-size">{m.size_display}</span>
                  {#if m.quantization}
                    <span class="model-pick-quant">{m.quantization}</span>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>

    {:else}
      <!-- Tabs -->
      <div class="tabs">
        <button class="tab" class:active={activeTab === "config"} onclick={() => activeTab = "config"}>配置</button>
        <button class="tab" class:active={activeTab === "logs"}   onclick={() => activeTab = "logs"}>日志</button>
        <div class="tab-spacer"></div>

        {#if !isCreating}
          {#if isRunning || isStarting}
            <button class="action-btn btn-stop" onclick={handleStop} disabled={saving || isStarting}>
              {isStarting ? "启动中..." : "停止"}
            </button>
          {:else}
            <button class="action-btn btn-start" onclick={handleStart} disabled={saving}>启动</button>
          {/if}
          <button class="action-btn btn-delete" onclick={handleDelete} disabled={saving}>删除</button>
        {:else}
          <button class="action-btn btn-ghost-action" onclick={() => { createStep = "model"; }}>← 重选模型</button>
          <button class="action-btn btn-start" onclick={handleSaveAndStart} disabled={saving || !editName || !editModelPath}>
            {saving ? "保存中..." : "保存并启动"}
          </button>
        {/if}
      </div>

      {#if actionErr}
        <div class="err-bar">{actionErr}</div>
      {/if}

      {#if activeTab === "config"}
        <InstanceEditForm
          {editName}
          {editModelPath}
          {editMode}
          {editParams}
          {isCreating}
          {scanning}
          selectedInfo={selectedInfo}
          onNameChange={(v) => { editName = v; }}
          onModelPathChange={(v) => { editModelPath = v; }}
          onModeChange={(v) => { editMode = v; }}
          onParamsChange={(p) => { editParams = p; }}
          onScan={handleScan}
        />
      {:else if activeTab === "logs"}
        <div class="log-area">
          {#if selectedName}
            <div class="log-toolbar">
              <span class="log-label">{selectedName} 进程日志</span>
              <button class="btn-ghost-sm" onclick={() => { if(selectedName) instanceStore.clearLogs(selectedName); }}>清空</button>
            </div>
            {@const displayParams = selectedInfo?.config?.params ?? selectedConfig?.params ?? null}
            {#if displayParams}
              {@const PARAM_LABELS: Record<string, string> = {
                gpu_layers: "GPU 层数",
                ctx_size: "上下文长度",
                threads: "线程数",
                flash_attn: "Flash Attention",
                cont_batching: "连续批处理",
                batch_size: "批处理大小",
                ubatch_size: "微批大小",
                parallel: "并行槽",
                cache_type_k: "KV缓存K类型",
                cache_type_v: "KV缓存V类型",
                no_kv_offload: "禁用KV卸载",
                seed: "随机种子",
                mlock: "内存锁定",
                no_mmap: "禁用内存映射",
                no_context_shift: "禁用上下文移位",
                keep: "保留头部Token数",
                extra_args: "额外参数",
              }}
              {@const activeParams = Object.entries(displayParams).filter(([, v]) => v !== null && v !== false)}
              {#if activeParams.length > 0}
                <div class="params-bar">
                  <span class="params-title">启动参数</span>
                  <button class="btn-ghost-sm" onclick={() => { paramsCollapsed = !paramsCollapsed; }}>
                    {paramsCollapsed ? "展开" : "收起"}
                  </button>
                </div>
                {#if !paramsCollapsed}
                  <div class="params-grid">
                    {#each activeParams as [key, val]}
                      {#if key === "extra_args"}
                        <div class="param-item param-full">
                          <span class="param-label">{PARAM_LABELS[key] ?? key}</span>
                          <span class="param-value">{val}</span>
                        </div>
                      {:else}
                        <div class="param-item">
                          <span class="param-label">{PARAM_LABELS[key] ?? key}</span>
                          <span class="param-value">{typeof val === "boolean" ? "✓" : val}</span>
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              {/if}
            {/if}
            <LogTerminal logs={selectedLogs} />
          {/if}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
.root {
  display: flex;
  height: 100%;
  overflow: hidden;
  background: var(--bg-base);
}

/* ── Left Panel ── */
.panel-left {
  width: 220px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  border-right: 1px solid var(--border-subtle);
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px 8px;
  flex-shrink: 0;
}

.panel-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
}

.btn-new {
  font-size: 11px;
  padding: 2px 8px;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.12s;
}
.btn-new:hover { opacity: 0.85; }

.instance-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.empty-hint {
  font-size: 11px;
  color: var(--text-muted);
  text-align: center;
  padding: 16px 8px;
}

.instance-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: none;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s, border-color 0.12s;
  width: 100%;
}
.instance-item:hover { background: var(--bg-hover); }
.instance-item.selected {
  background: rgba(59,130,246,0.09);
  border-color: rgba(59,130,246,0.2);
}
.instance-item.creating {
  opacity: 0.7;
  border-style: dashed;
  border-color: var(--accent);
}

.inst-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

.inst-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.inst-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.inst-model {
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.inst-status {
  font-size: 10px;
  flex-shrink: 0;
}

/* ── Left Panel Footer ── */
.panel-footer {
  flex-shrink: 0;
  padding: 6px 8px;
  border-top: 1px solid var(--border-subtle);
}

.proxy-log-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 6px 8px;
  border-radius: 5px;
  border: 1px solid transparent;
  background: none;
  cursor: pointer;
  font-size: 11px;
  color: var(--text-muted);
  transition: background 0.12s, color 0.12s;
  text-align: left;
}
.proxy-log-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
.proxy-log-btn.active {
  background: rgba(59,130,246,0.09);
  border-color: rgba(59,130,246,0.2);
  color: var(--text-base);
}

.proxy-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--border);
  flex-shrink: 0;
  transition: background 0.2s;
}
.proxy-dot.has-logs { background: var(--accent); }

.proxy-count {
  margin-left: auto;
  font-size: 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  padding: 0 5px;
  line-height: 16px;
  color: var(--text-muted);
}

/* ── Right Panel ── */
.panel-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-base);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
}
.empty-icon { font-size: 32px; opacity: 0.3; }
.empty-msg { font-size: 12px; }

/* ── Tabs ── */
.tabs {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
  height: 36px;
}

.tab {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 0 8px;
  height: 36px;
  cursor: pointer;
  transition: color 0.12s, border-color 0.12s;
}
.tab:hover { color: var(--text-secondary); }
.tab.active { color: var(--text-base); border-bottom-color: var(--accent); }

.tab-spacer { flex: 1; }

.action-btn {
  font-size: 11px;
  font-weight: 500;
  height: 24px;
  padding: 0 12px;
  border-radius: 4px;
  border: none;
  cursor: pointer;
  margin-left: 4px;
  transition: opacity 0.12s;
}
.action-btn:disabled { opacity: 0.45; cursor: not-allowed; }

.btn-start  { background: var(--accent);   color: #fff; }
.btn-stop   { background: var(--warning);  color: #fff; }
.btn-delete { background: transparent; color: var(--danger); border: 1px solid rgba(239,68,68,0.3); }
.btn-start:not(:disabled):hover,
.btn-stop:not(:disabled):hover  { opacity: 0.85; }
.btn-delete:not(:disabled):hover { background: rgba(239,68,68,0.08); }

.err-bar {
  padding: 6px 12px;
  font-size: 11px;
  color: var(--danger);
  background: rgba(239,68,68,0.07);
  border-bottom: 1px solid rgba(239,68,68,0.15);
  flex-shrink: 0;
}

/* ── Model pick panel ── */
.model-pick-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.model-pick-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.model-pick-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-base);
  flex: 1;
}

.model-pick-loading,
.model-pick-empty {
  padding: 24px 16px;
  font-size: 12px;
  color: var(--text-muted);
  text-align: center;
}

.model-pick-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.model-pick-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: none;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s, border-color 0.12s;
}
.model-pick-item:hover {
  background: var(--bg-hover);
  border-color: var(--border-subtle);
}

.model-pick-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-base);
  word-break: break-all;
}

.model-pick-meta {
  display: flex;
  gap: 8px;
  align-items: center;
}

.model-pick-size {
  font-size: 10px;
  color: var(--text-muted);
}

.model-pick-quant {
  font-size: 10px;
  color: var(--accent);
  background: rgba(59,130,246,0.1);
  border-radius: 3px;
  padding: 0 4px;
}

.btn-ghost-action {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border-subtle);
}
.btn-ghost-action:hover { background: var(--bg-hover); }

/* ── Log area ── */
.log-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.log-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.log-label {
  font-size: 11px;
  color: var(--text-muted);
}

.params-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 5px 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.params-title {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 500;
}

.params-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 4px 8px;
  padding: 8px 12px;
  background: var(--bg-base);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.param-item {
  display: flex;
  align-items: baseline;
  gap: 4px;
  min-width: 0;
}

.param-item.param-full {
  grid-column: 1 / -1;
}

.param-label {
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
  flex-shrink: 0;
}

.param-value {
  font-size: 11px;
  color: var(--text-primary);
  font-family: var(--font-mono, monospace);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-ghost {
  height: 26px;
  padding: 0 10px;
  font-size: 11px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: background 0.12s;
}
.btn-ghost:hover { background: var(--bg-hover); }
.btn-ghost:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-ghost-sm {
  font-size: 10px;
  padding: 2px 6px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
}
.btn-ghost-sm:hover { background: var(--bg-hover); }
</style>
