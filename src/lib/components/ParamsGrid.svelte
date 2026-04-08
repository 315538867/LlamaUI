<script lang="ts">
  import type { LaunchParams } from "../types";
  import { PARAM_LABELS } from "../types";

  interface Props {
    params: LaunchParams | null;
  }

  const { params }: Props = $props();

  let collapsed = $state(false);

  const activeParams = $derived(
    params ? Object.entries(params).filter(([, v]) => v !== null && v !== false) : []
  );
</script>

{#if activeParams.length > 0}
  <div class="params-bar">
    <span class="params-title">启动参数</span>
    <button class="btn-ghost-sm" onclick={() => { collapsed = !collapsed; }}>
      {collapsed ? "展开" : "收起"}
    </button>
  </div>
  {#if !collapsed}
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

<style>
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
