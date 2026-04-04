<script lang="ts">
  import { getProcessStore } from "../stores/process.svelte";

  const process = getProcessStore();

  let logContainer: HTMLDivElement;
  let autoScroll = $state(true);

  $effect(() => {
    if (autoScroll && logContainer && process.logs.length > 0) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });
</script>

<div
  class="flex h-full flex-col overflow-hidden rounded-lg border"
  style="background:#0d0d0f; border-color:var(--border-subtle);"
>
  <!-- 工具栏 -->
  <div
    class="flex shrink-0 items-center justify-between border-b px-3 py-1.5"
    style="border-color:var(--border-subtle); background:var(--bg-surface);"
  >
    <div class="flex items-center gap-2">
      <span class="text-[11px] font-medium" style="color:var(--text-secondary);">日志输出</span>
      {#if process.logs.length > 0}
        <span
          class="rounded px-1.5 py-0.5 text-[10px]"
          style="background:var(--bg-overlay); color:var(--text-muted);"
        >{process.logs.length}</span>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      <label class="flex cursor-pointer items-center gap-1.5 text-[11px]" style="color:var(--text-muted);">
        <input type="checkbox" bind:checked={autoScroll} class="h-3 w-3 accent-blue-500" />
        自动滚动
      </label>
      <button
        onclick={() => process.clearLogs()}
        class="rounded px-2 py-0.5 text-[11px] transition-colors"
        style="color:var(--text-muted);"
        onmouseenter={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-base)")}
        onmouseleave={(e) => ((e.currentTarget as HTMLElement).style.color = "var(--text-muted)")}
      >
        清除
      </button>
    </div>
  </div>

  <!-- 日志内容 -->
  <div bind:this={logContainer} class="flex-1 overflow-y-auto p-3 font-mono text-[11px] leading-5">
    {#if process.logs.length === 0}
      <p style="color:var(--text-muted);">等待启动...</p>
    {:else}
      {#each process.logs as log}
        <div style={log.stream === "stderr" ? "color:var(--warning);" : "color:#6b7280;"}>
          {log.line}
        </div>
      {/each}
    {/if}
  </div>
</div>
