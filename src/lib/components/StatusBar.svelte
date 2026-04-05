<script lang="ts">
  import { getProcessStore } from "../stores/process.svelte";
  import { onMount } from "svelte";

  const process = getProcessStore();

  const statusColor = $derived(
    process.info.status === "running"  ? "var(--success)"
    : process.info.status === "starting" ? "var(--warning)"
    : process.info.status === "error"    ? "var(--danger)"
    : "var(--text-muted)"
  );

  const statusText = $derived(
    process.info.status === "running"  ? "运行中"
    : process.info.status === "starting" ? "启动中"
    : process.info.status === "error"    ? "错误"
    : "已停止"
  );

  let now = $state(Math.floor(Date.now() / 1000));
  onMount(() => {
    const t = setInterval(() => { now = Math.floor(Date.now() / 1000); }, 1000);
    return () => clearInterval(t);
  });

  const uptime = $derived.by(() => {
    if (!process.info.started_at) return "";
    const s = now - process.info.started_at;
    if (s < 0) return "";
    const m = Math.floor(s / 60), h = Math.floor(m / 60);
    if (h > 0) return `${h}h ${m % 60}m`;
    if (m > 0) return `${m}m ${s % 60}s`;
    return `${s}s`;
  });
</script>

<footer class="statusbar">
  <!-- 左侧：状态 + 模型 -->
  <div class="left">
    <span class="status-dot" style="background:{statusColor}; box-shadow:0 0 5px {statusColor};"></span>
    <span class="status-text">{statusText}</span>

    {#if process.info.model}
      <span class="sep">|</span>
      <span class="model-name">
        {process.info.model.split(/[/\\]/).pop()}
      </span>
    {/if}
  </div>

  <!-- 右侧：性能指标 -->
  <div class="right">
    {#if process.tokensPerSec != null}
      <span class="tps">⚡ {process.tokensPerSec.toFixed(1)} t/s</span>
    {/if}
    {#if process.promptTps != null && process.tokensPerSec != null}
      <span class="sep">|</span>
      <span class="stat" title="Prefill 速度">↑ {process.promptTps.toFixed(0)} t/s</span>
    {/if}
    {#if process.info.port}
      <span class="sep">|</span>
      <span class="stat">:{process.info.port}</span>
    {/if}
    {#if uptime}
      <span class="sep">|</span>
      <span class="stat">{uptime}</span>
    {/if}
    {#if process.info.pid}
      <span class="sep">|</span>
      <span class="stat">PID {process.info.pid}</span>
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

/* ─ Left ─ */
.left {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
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

.model-name {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 200px;
}

/* ─ Right ─ */
.right {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-shrink: 0;
}

.stat {
  font-size: 11px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.tps {
  font-size: 11px;
  font-weight: 600;
  color: var(--success);
  font-variant-numeric: tabular-nums;
}

/* ─ Separator ─ */
.sep {
  font-size: 10px;
  color: var(--border);
  line-height: 1;
  flex-shrink: 0;
}
</style>
