<script lang="ts">
  import { getInstanceStore } from "../stores/process.svelte";
  import type { InstanceInfo } from "../types";
  import { onMount } from "svelte";

  const store = getInstanceStore();

  const runningInstances = $derived(
    Object.values(store.instances).filter((i: InstanceInfo) => i.status === "running")
  );

  const statusColor = $derived(
    runningInstances.length > 0 ? "var(--success)"
    : Object.values(store.instances).some((i: InstanceInfo) => i.status === "starting") ? "var(--warning)"
    : Object.values(store.instances).some((i: InstanceInfo) => i.status === "error") ? "var(--danger)"
    : "var(--text-muted)"
  );

  const statusText = $derived(
    runningInstances.length > 0 ? `${runningInstances.length} 实例运行中`
    : Object.values(store.instances).some((i: InstanceInfo) => i.status === "starting") ? "启动中..."
    : "已停止"
  );

  let now = $state(Math.floor(Date.now() / 1000));
  onMount(() => {
    const t = setInterval(() => { now = Math.floor(Date.now() / 1000); }, 1000);
    return () => clearInterval(t);
  });

  function fmtTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return String(n);
  }
</script>

<footer class="statusbar">
  <div class="left">
    <span class="status-dot" style="background:{statusColor}; box-shadow:0 0 5px {statusColor};"></span>
    <span class="status-text">{statusText}</span>

    {#if runningInstances.length > 0}
      <span class="sep">|</span>
      <span class="instance-list">
        {#each runningInstances as inst (inst.config.name)}
          {@const perf = store.getInstancePerf(inst.config.name)}
          <span class="instance-chip">
            <span class="chip-name">{inst.config.name}</span>
            {#if perf?.evalTps != null}
              <span class="chip-tps">⚡{perf.evalTps.toFixed(1)}</span>
            {/if}
            {#if perf && (perf.totalPromptTokens > 0 || perf.totalEvalTokens > 0)}
              <span class="chip-tokens">{fmtTokens(perf.totalPromptTokens)}↑ {fmtTokens(perf.totalEvalTokens)}↓</span>
            {/if}
          </span>
        {/each}
      </span>
    {/if}
  </div>

  <div class="right">
    {#if store.tokensPerSec != null}
      <span class="tps">⚡ {store.tokensPerSec.toFixed(1)} t/s</span>
    {/if}
  </div>
</footer>

<style>
.statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 26px;
  padding: 0 12px;
  flex-shrink: 0;
  background: var(--bg-surface);
  border-top: 1px solid var(--border-subtle);
  user-select: none;
}

.left {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  overflow: hidden;
}

.status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-text {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  flex-shrink: 0;
}

.instance-list {
  display: flex;
  gap: 4px;
  overflow: hidden;
}

.instance-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  padding: 0 5px;
  line-height: 16px;
  white-space: nowrap;
}

.chip-name {
  color: var(--text-secondary);
  font-weight: 500;
}

.chip-tps {
  color: var(--success);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.chip-tokens {
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  opacity: 0.8;
}

.right {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-shrink: 0;
}

.tps {
  font-size: 11px;
  font-weight: 600;
  color: var(--success);
  font-variant-numeric: tabular-nums;
}

.sep {
  font-size: 10px;
  color: var(--border);
  line-height: 1;
  flex-shrink: 0;
}
</style>
