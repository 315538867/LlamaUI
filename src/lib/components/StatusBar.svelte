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

<footer
  class="flex h-6 shrink-0 items-center justify-between border-t px-3"
  style="background:var(--bg-surface); border-color:var(--border-subtle);"
>
  <div class="flex items-center gap-2">
    <!-- 状态指示点 -->
    <span
      class="inline-block h-1.5 w-1.5 rounded-full"
      style="background:{statusColor}; box-shadow: 0 0 4px {statusColor};"
    ></span>
    <span class="text-[11px]" style="color:var(--text-secondary);">{statusText}</span>

    {#if process.info.model}
      <span class="text-[11px]" style="color:var(--border);">·</span>
      <span class="max-w-52 truncate text-[11px]" style="color:var(--text-muted);">
        {process.info.model.split(/[/\\]/).pop()}
      </span>
    {/if}
  </div>

  <div class="flex items-center gap-3">
    {#if process.info.port}
      <span class="text-[11px]" style="color:var(--text-muted);">:{process.info.port}</span>
    {/if}
    {#if uptime}
      <span class="text-[11px]" style="color:var(--text-muted);">{uptime}</span>
    {/if}
    {#if process.info.pid}
      <span class="text-[11px]" style="color:var(--text-muted);">PID {process.info.pid}</span>
    {/if}
  </div>
</footer>
