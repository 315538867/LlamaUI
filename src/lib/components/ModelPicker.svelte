<script lang="ts">
  import { getModelStore } from "../stores/models.svelte";
  import type { ModelInfo } from "../types";

  interface Props {
    onPick: (model: ModelInfo) => void;
    onCancel: () => void;
  }

  const { onPick, onCancel }: Props = $props();

  const modelStore = getModelStore();
</script>

<div class="model-pick-panel">
  <div class="model-pick-header">
    <span class="model-pick-title">选择模型文件</span>
    <button class="btn-ghost" onclick={() => modelStore.refresh()} disabled={modelStore.loading}>
      {modelStore.loading ? "扫描中..." : "重新扫描"}
    </button>
    <button class="btn-ghost" onclick={onCancel}>取消</button>
  </div>

  {#if modelStore.loading}
    <div class="model-pick-loading">扫描中...</div>
  {:else if modelStore.models.length === 0}
    <div class="model-pick-empty">
      未找到模型文件，请先在设置中配置模型目录
    </div>
  {:else}
    <div class="model-pick-list">
      {#each modelStore.models as m}
        <button class="model-pick-item" onclick={() => onPick(m)}>
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

<style>
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
</style>
