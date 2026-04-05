<script lang="ts">
  import { onDestroy } from "svelte";

  interface LogEntry {
    stream: string;
    line: string;
    ts?: number;
  }

  interface Props {
    logs: LogEntry[];
  }

  let { logs }: Props = $props();

  let logContainer: HTMLDivElement;
  let autoScroll = $state(true);
  let rafId: number | undefined;

  $effect(() => {
    if (autoScroll && logContainer && logs.length > 0) {
      if (rafId !== undefined) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        logContainer.scrollTop = logContainer.scrollHeight;
        rafId = undefined;
      });
    }
    return () => {
      if (rafId !== undefined) cancelAnimationFrame(rafId);
    };
  });

  onDestroy(() => {
    if (rafId !== undefined) cancelAnimationFrame(rafId);
  });
</script>

<div class="terminal">
  <div bind:this={logContainer} class="log-scroll">
    {#if logs.length === 0}
      <p class="empty">等待输出...</p>
    {:else}
      {#each logs as log}
        <div class="log-line" class:stderr={log.stream === "stderr"}>{log.line}</div>
      {/each}
    {/if}
  </div>
</div>

<style>
.terminal {
  background: #0d0d0f;
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.log-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  font-family: monospace;
  font-size: 11px;
  line-height: 1.6;
}

.log-line {
  color: #6b7280;
  word-break: break-all;
  white-space: pre-wrap;
}

.log-line.stderr {
  color: var(--warning);
}

.empty {
  color: var(--text-muted);
  font-size: 11px;
}
</style>
