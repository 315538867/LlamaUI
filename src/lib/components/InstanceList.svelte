<script lang="ts">
  import { getInstanceStore } from "../stores/process.svelte";
  import type { InstanceConfig, InstanceInfo } from "../types";

  interface Props {
    savedInstances: InstanceConfig[];
    selectedName: string | null;
    isCreating: boolean;
    creatingName: string;
    onSelect: (name: string) => void;
    onNew: () => void;
  }

  const { savedInstances, selectedName, isCreating, creatingName, onSelect, onNew }: Props = $props();

  const instanceStore = getInstanceStore();

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

<div class="panel-left">
  <div class="panel-header">
    <span class="panel-title">模型实例</span>
    <button class="btn-new" onclick={onNew}>+ 新建</button>
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
        onclick={() => onSelect(cfg.name)}
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
          <span class="inst-name">{creatingName || "新实例..."}</span>
          <span class="inst-model">未保存</span>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
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
</style>
