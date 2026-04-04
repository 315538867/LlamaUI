<script lang="ts">
  import { getProcessStore } from "../stores/process.svelte";
  import { onMount } from "svelte";

  const process = getProcessStore();

  const statusColor = $derived(
    process.info.status === "running"
      ? "var(--success)"
      : process.info.status === "starting"
        ? "var(--warning)"
        : process.info.status === "error"
          ? "var(--danger)"
          : "var(--text-muted)"
  );

  const statusText = $derived(
    process.info.status === "running"
      ? "运行中"
      : process.info.status === "starting"
        ? "启动中..."
        : process.info.status === "error"
          ? "错误"
          : "已停止"
  );

  // Ticking timer for uptime display (#17 fix)
  let now = $state(Math.floor(Date.now() / 1000));

  onMount(() => {
    const interval = setInterval(() => {
      now = Math.floor(Date.now() / 1000);
    }, 1000);
    return () => clearInterval(interval);
  });

  const uptime = $derived.by(() => {
    if (!process.info.started_at) return "";
    const seconds = now - process.info.started_at;
    if (seconds < 0) return "";
    const m = Math.floor(seconds / 60);
    const h = Math.floor(m / 60);
    if (h > 0) return `${h}h ${m % 60}m`;
    if (m > 0) return `${m}m ${seconds % 60}s`;
    return `${seconds}s`;
  });
</script>

<footer class="flex h-7 items-center justify-between border-t border-[var(--border-color)] bg-[var(--bg-secondary)] px-3 text-xs">
  <div class="flex items-center gap-2">
    <span
      class="inline-block h-2 w-2 rounded-full"
      style="background-color: {statusColor}"
    ></span>
    <span class="text-[var(--text-secondary)]">{statusText}</span>
    {#if process.info.model}
      <span class="text-[var(--text-muted)]">|</span>
      <span class="max-w-48 truncate text-[var(--text-secondary)]">
        {process.info.model?.split(/[/\\]/).pop()}
      </span>
    {/if}
  </div>

  <div class="flex items-center gap-3 text-[var(--text-muted)]">
    {#if process.info.port}
      <span>:{process.info.port}</span>
    {/if}
    {#if uptime}
      <span>{uptime}</span>
    {/if}
    {#if process.info.pid}
      <span>PID {process.info.pid}</span>
    {/if}
  </div>
</footer>
