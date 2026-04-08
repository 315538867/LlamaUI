<script lang="ts">
  import { getInstanceStore } from "../stores/process.svelte";
  import { getConfigStore } from "../stores/config.svelte";
  import {
    startInstance,
    stopInstance,
    deleteInstanceConfig,
  } from "../services/tauri-bridge";
  import type { InstanceConfig, InstanceInfo, ModelInfo, LaunchParams } from "../types";
  import { DEFAULT_PARAMS } from "../types";
  import LogTerminal from "./LogTerminal.svelte";
  import InstanceEditForm from "./InstanceEditForm.svelte";
  import InstanceList from "./InstanceList.svelte";
  import ModelPicker from "./ModelPicker.svelte";
  import ParamsGrid from "./ParamsGrid.svelte";

  const instanceStore = getInstanceStore();
  const configStore = getConfigStore();

  // ── State ─────────────────────────────────────────────────────────────────

  let selectedName = $state<string | null>(null);
  let isCreating = $state(false);
  let createStep = $state<"model" | "config">("model");
  let activeTab = $state<"config" | "logs">("config");
  let saving = $state(false);
  let actionErr = $state("");
  let actionErrTimer: ReturnType<typeof setTimeout> | undefined;
  let deleteConfirmName = $state<string | null>(null);

  // Edit form state (lifted here, passed down to InstanceEditForm)
  let editName = $state("");
  let editModelPath = $state("");
  let editMode = $state<"server" | "cli">("server");
  let editParams = $state<LaunchParams>({ ...DEFAULT_PARAMS });

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

  const displayParams = $derived(
    selectedInfo?.config?.params ?? selectedConfig?.params ?? null
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
    const cfg = savedInstances.find((i) => i.name === name);
    if (cfg) loadFormFrom(cfg);
  }

  function startCreate() {
    isCreating = true;
    createStep = "model";
    selectedName = null;
    editName = "";
    editModelPath = "";
    editMode = "server";
    editParams = { ...DEFAULT_PARAMS };
    activeTab = "config";
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
      activeTab = "logs"; // 首次创建，切换到日志
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
    deleteConfirmName = selectedName;
  }

  async function confirmDelete() {
    if (!deleteConfirmName) return;
    const name = deleteConfirmName;
    deleteConfirmName = null;
    saving = true;
    actionErr = "";
    try {
      const info = instanceStore.instances[name];
      if (info?.status === "running") await stopInstance(name);
      await deleteInstanceConfig(name);
      await configStore.load();
      selectedName = null;
      isCreating = false;
    } catch (e) { showErr(e); }
    finally { saving = false; }
  }
</script>

<div class="root">
  <InstanceList
    {savedInstances}
    {selectedName}
    {isCreating}
    creatingName={editName}
    onSelect={selectInstance}
    onNew={startCreate}
  />

  <!-- ── Right: Content panel ─────────────────────────────────────────────── -->
  <div class="panel-right">
    {#if !selectedName && !isCreating}
      <div class="empty-state">
        <div class="empty-icon">⬡</div>
        <div class="empty-msg">选择或创建一个实例</div>
      </div>

    {:else if isCreating && createStep === "model"}
      <ModelPicker
        onPick={pickModel}
        onCancel={() => { isCreating = false; }}
      />

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

      {#if deleteConfirmName}
        <div class="confirm-bar">
          <span>确认删除实例「{deleteConfirmName}」？此操作不可撤销。</span>
          <button class="action-btn btn-delete" onclick={confirmDelete} disabled={saving}>确认删除</button>
          <button class="btn-ghost-sm" onclick={() => { deleteConfirmName = null; }}>取消</button>
        </div>
      {/if}

      {#if activeTab === "config"}
        <InstanceEditForm
          {editName}
          {editModelPath}
          {editMode}
          {editParams}
          {isCreating}
          selectedInfo={selectedInfo}
          onNameChange={(v) => { editName = v; }}
          onModelPathChange={(v) => { editModelPath = v; }}
          onModeChange={(v) => { editMode = v; }}
          onParamsChange={(p) => { editParams = p; }}
        />
      {:else if activeTab === "logs"}
        <div class="log-area">
          {#if selectedName}
            <div class="log-toolbar">
              <span class="log-label">{selectedName} 进程日志</span>
              <button class="btn-ghost-sm" onclick={() => { if (selectedName) instanceStore.clearLogs(selectedName); }}>清空</button>
            </div>
            <ParamsGrid params={displayParams} />
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

.btn-ghost-action {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border-subtle);
}
.btn-ghost-action:hover { background: var(--bg-hover); }

.err-bar {
  padding: 6px 12px;
  font-size: 11px;
  color: var(--danger);
  background: rgba(239,68,68,0.07);
  border-bottom: 1px solid rgba(239,68,68,0.15);
  flex-shrink: 0;
}

.confirm-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  font-size: 11px;
  color: var(--text-secondary);
  background: rgba(239,68,68,0.06);
  border-bottom: 1px solid rgba(239,68,68,0.18);
  flex-shrink: 0;
}
.confirm-bar span { flex: 1; }

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
