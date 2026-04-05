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
  import LogTerminal from "./LogTerminal.svelte";

  const instanceStore = getInstanceStore();
  const configStore = getConfigStore();
  const proxyStore = getProxyStore();

  // ── State ─────────────────────────────────────────────────────────────────

  let selectedName = $state<string | null>(null);
  let isCreating = $state(false);
  let activeTab = $state<"config" | "logs" | "proxy">("config");
  let availableModels = $state<ModelInfo[]>([]);
  let scanning = $state(false);
  let saving = $state(false);
  let actionErr = $state("");
  let actionErrTimer: ReturnType<typeof setTimeout> | undefined;

  // Edit form
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
      api_key: null,
      system_prompt: null,
      extra_args: null,
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
    const cfg = savedInstances.find((i) => i.name === name);
    if (cfg) loadFormFrom(cfg);
  }

  function startCreate() {
    isCreating = true;
    selectedName = null;
    editName = "";
    editModelPath = "";
    editMode = "server";
    editParams = defaultParams();
    activeTab = "config";
    // Scan models if not yet done
    if (availableModels.length === 0) handleScan();
  }

  async function handleScan() {
    scanning = true;
    try {
      const result = await scanModels();
      availableModels = result.models;
    } catch (e) {
      console.error(e);
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
      // Config is persisted by backend; reload config to refresh saved list
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

  // Per-instance logs formatted for LogTerminal
  const selectedLogs = $derived(
    selectedName ? instanceStore.getInstanceLogs(selectedName) : []
  );

  const proxyLogLines = $derived(
    proxyStore.logs.map((e) => ({
      stream: e.level === "error" ? "stderr" : "stdout",
      line: `[${e.level.toUpperCase()}] ${e.message}`,
      ts: e.timestamp,
    }))
  );
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
  </div>

  <!-- ── Right: Config / Log panel ────────────────────────────────────────── -->
  <div class="panel-right">
    {#if !selectedName && !isCreating}
      <div class="empty-state">
        <div class="empty-icon">⬡</div>
        <div class="empty-msg">选择或创建一个实例</div>
      </div>
    {:else}
      <!-- Tabs -->
      <div class="tabs">
        <button class="tab" class:active={activeTab === "config"} onclick={() => activeTab = "config"}>配置</button>
        <button class="tab" class:active={activeTab === "logs"}   onclick={() => activeTab = "logs"}>日志</button>
        <button class="tab" class:active={activeTab === "proxy"}  onclick={() => activeTab = "proxy"}>代理日志</button>
        <div class="tab-spacer"></div>

        <!-- Actions -->
        {#if !isCreating}
          {#if isRunning || isStarting}
            <button class="action-btn btn-stop" onclick={handleStop} disabled={saving || isStarting}>
              {isStarting ? "启动中..." : "停止"}
            </button>
          {:else}
            <button class="action-btn btn-start" onclick={handleStart} disabled={saving}>启动</button>
          {/if}
          <button class="action-btn btn-delete" onclick={handleDelete} disabled={saving || isRunning}>删除</button>
        {:else}
          <button class="action-btn btn-start" onclick={handleSaveAndStart} disabled={saving || !editName || !editModelPath}>
            {saving ? "保存中..." : "保存并启动"}
          </button>
        {/if}
      </div>

      {#if actionErr}
        <div class="err-bar">{actionErr}</div>
      {/if}

      <!-- Config Tab -->
      {#if activeTab === "config"}
        <div class="config-body">

          <!-- Instance name -->
          <div class="field-row">
            <label class="field-label" for="edit-name">实例名称</label>
            <input id="edit-name" class="field-input" type="text" bind:value={editName}
              placeholder="my-model（作为 Codex model 字段）"
              disabled={!isCreating}
            />
          </div>
          <div class="field-hint">此名称将作为 Codex 请求中的 <code>model</code> 字段路由到此实例</div>

          <!-- Model path -->
          <div class="field-row">
            <label class="field-label" for="edit-model">模型文件</label>
            <div class="model-picker">
              {#if availableModels.length > 0}
                <select id="edit-model" class="field-select" bind:value={editModelPath}>
                  <option value="">— 选择模型 —</option>
                  {#each availableModels as m}
                    <option value={m.path}>{m.name} ({m.size_display})</option>
                  {/each}
                </select>
              {:else}
                <input id="edit-model" class="field-input flex-1" type="text" bind:value={editModelPath}
                  placeholder="/path/to/model.gguf" />
              {/if}
              <button class="btn-ghost" onclick={handleScan} disabled={scanning}>
                {scanning ? "扫描中..." : "扫描"}
              </button>
            </div>
          </div>

          <!-- Mode -->
          <div class="field-row">
            <label class="field-label" for="edit-mode">运行模式</label>
            <select id="edit-mode" class="field-select" bind:value={editMode}>
              <option value="server">server（HTTP API）</option>
              <option value="cli">cli（交互式）</option>
            </select>
          </div>

          <div class="section-divider">参数</div>

          <!-- GPU layers -->
          <div class="field-row">
            <label class="field-label" for="edit-gpu">GPU 层数</label>
            <input id="edit-gpu" class="field-input w-num" type="number"
              value={editParams.gpu_layers ?? ""}
              oninput={(e) => { editParams.gpu_layers = (e.target as HTMLInputElement).value === "" ? null : parseInt((e.target as HTMLInputElement).value); }}
              placeholder="99"
            />
          </div>

          <!-- Context size -->
          <div class="field-row">
            <label class="field-label" for="edit-ctx">上下文大小</label>
            <input id="edit-ctx" class="field-input w-num" type="number"
              value={editParams.ctx_size ?? ""}
              oninput={(e) => { editParams.ctx_size = (e.target as HTMLInputElement).value === "" ? null : parseInt((e.target as HTMLInputElement).value); }}
              placeholder="4096"
            />
          </div>

          <!-- Parallel -->
          <div class="field-row">
            <label class="field-label" for="edit-parallel">并发槽数</label>
            <input id="edit-parallel" class="field-input w-num" type="number"
              value={editParams.parallel ?? ""}
              oninput={(e) => { editParams.parallel = (e.target as HTMLInputElement).value === "" ? null : parseInt((e.target as HTMLInputElement).value); }}
              placeholder="1"
            />
          </div>

          <!-- Toggles -->
          <div class="field-row">
            <span class="field-label">Flash Attention</span>
            <label class="toggle">
              <input type="checkbox"
                checked={editParams.flash_attn ?? false}
                onchange={(e) => { editParams.flash_attn = (e.target as HTMLInputElement).checked; }}
              />
              <span class="toggle-track"></span>
            </label>
          </div>

          <div class="field-row">
            <span class="field-label">持续批处理</span>
            <label class="toggle">
              <input type="checkbox"
                checked={editParams.cont_batching ?? false}
                onchange={(e) => { editParams.cont_batching = (e.target as HTMLInputElement).checked; }}
              />
              <span class="toggle-track"></span>
            </label>
          </div>

          <!-- Instance API key -->
          <div class="field-row">
            <label class="field-label" for="edit-apikey">实例 API Key</label>
            <input id="edit-apikey" class="field-input flex-1" type="password"
              value={editParams.api_key ?? ""}
              oninput={(e) => { editParams.api_key = (e.target as HTMLInputElement).value || null; }}
              placeholder="可选，传入 --api-key"
            />
          </div>

          <!-- Extra args -->
          <div class="field-row">
            <label class="field-label" for="edit-extra">额外参数</label>
            <input id="edit-extra" class="field-input flex-1" type="text"
              value={editParams.extra_args ?? ""}
              oninput={(e) => { editParams.extra_args = (e.target as HTMLInputElement).value || null; }}
              placeholder="如 --no-mmap --mlock"
            />
          </div>

          <!-- Info row when running -->
          {#if selectedInfo?.status === "running"}
            <div class="running-info">
              <span>端口 <code>:{selectedInfo.port}</code></span>
              <span>PID <code>{selectedInfo.pid}</code></span>
            </div>
          {/if}
        </div>

      <!-- Log Tab -->
      {:else if activeTab === "logs"}
        <div class="log-area">
          {#if selectedName}
            <div class="log-toolbar">
              <span class="log-label">{selectedName} 进程日志</span>
              <button class="btn-ghost-sm" onclick={() => { if(selectedName) instanceStore.clearLogs(selectedName); }}>清空</button>
            </div>
            <LogTerminal logs={selectedLogs} />
          {/if}
        </div>

      <!-- Proxy Log Tab -->
      {:else if activeTab === "proxy"}
        <div class="log-area">
          <div class="log-toolbar">
            <span class="log-label">代理转发日志</span>
            <button class="btn-ghost-sm" onclick={() => proxyStore.clearLogs()}>清空</button>
          </div>
          <LogTerminal logs={proxyLogLines} />
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

/* ── Config body ── */
.config-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-divider {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--text-muted);
  border-top: 1px solid var(--border-subtle);
  padding-top: 8px;
  margin-top: 4px;
}

.field-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 28px;
}

.field-label {
  font-size: 11px;
  color: var(--text-secondary);
  width: 100px;
  flex-shrink: 0;
}

.field-input {
  height: 26px;
  padding: 0 8px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  transition: border-color 0.12s;
  min-width: 0;
}
.field-input:focus { border-color: var(--accent); }
.field-input.flex-1 { flex: 1; }
.field-input.w-num { width: 80px; }

.field-select {
  height: 26px;
  padding: 0 6px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  cursor: pointer;
  flex: 1;
}

.model-picker {
  display: flex;
  gap: 6px;
  align-items: center;
  flex: 1;
  min-width: 0;
}

.field-hint {
  font-size: 10px;
  color: var(--text-muted);
  padding-left: 110px;
  line-height: 1.4;
}
.field-hint code {
  font-family: monospace;
  background: var(--bg-overlay);
  padding: 0 3px;
  border-radius: 2px;
}

/* Toggle */
.toggle {
  display: flex;
  align-items: center;
  cursor: pointer;
  position: relative;
}
.toggle input { position: absolute; opacity: 0; width: 0; height: 0; }
.toggle-track {
  width: 32px;
  height: 18px;
  border-radius: 9px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  transition: background 0.15s, border-color 0.15s;
  position: relative;
}
.toggle-track::after {
  content: "";
  position: absolute;
  left: 2px;
  top: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: left 0.15s, background 0.15s;
}
.toggle input:checked + .toggle-track {
  background: rgba(59,130,246,0.25);
  border-color: var(--accent);
}
.toggle input:checked + .toggle-track::after {
  left: 16px;
  background: var(--accent);
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

.running-info {
  display: flex;
  gap: 16px;
  padding: 8px 0 0;
  font-size: 11px;
  color: var(--text-muted);
  border-top: 1px solid var(--border-subtle);
  margin-top: 4px;
}
.running-info code {
  font-family: monospace;
  color: var(--text-secondary);
}

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
