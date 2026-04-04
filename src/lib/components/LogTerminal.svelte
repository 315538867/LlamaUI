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

<div class="flex h-full flex-col overflow-hidden rounded-lg border border-[var(--border-color)] bg-[var(--bg-primary)]">
  <div class="flex items-center justify-between border-b border-[var(--border-color)] px-3 py-1.5">
    <span class="text-xs font-medium text-[var(--text-secondary)]">日志输出</span>
    <div class="flex items-center gap-2">
      <label class="flex items-center gap-1 text-xs text-[var(--text-muted)]">
        <input type="checkbox" bind:checked={autoScroll} class="h-3 w-3" />
        自动滚动
      </label>
      <button
        class="rounded px-2 py-0.5 text-xs text-[var(--text-muted)] transition-colors hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
        onclick={() => process.clearLogs()}
      >
        清除
      </button>
    </div>
  </div>
  <div
    bind:this={logContainer}
    class="flex-1 overflow-y-auto p-2 font-mono text-xs leading-5"
  >
    {#if process.logs.length === 0}
      <p class="text-[var(--text-muted)]">等待启动...</p>
    {:else}
      {#each process.logs as log}
        <div class={log.stream === "stderr" ? "text-[var(--warning)]" : "text-[var(--text-secondary)]"}>
          {log.line}
        </div>
      {/each}
    {/if}
  </div>
</div>
