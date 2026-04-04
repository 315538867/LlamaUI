<script lang="ts">
  import type { PageId } from "../types";

  interface Props {
    activePage: PageId;
    onNavigate: (page: PageId) => void;
  }

  let { activePage, onNavigate }: Props = $props();

  const navItems: { id: PageId; label: string; icon: string }[] = [
    { id: "launcher", label: "启动器", icon: "▶" },
    { id: "models",   label: "模型库", icon: "◈" },
    { id: "settings", label: "设置",   icon: "⚙" },
  ];
</script>

<nav class="sidebar flex h-full w-44 shrink-0 flex-col border-r py-3">
  <!-- Logo -->
  <div class="mb-5 px-4">
    <div class="flex items-center gap-2">
      <div class="logo flex h-7 w-7 items-center justify-center rounded-lg text-sm font-bold text-white">L</div>
      <span class="text-sm font-semibold" style="color:var(--text-base);">LlamaUI</span>
    </div>
  </div>

  <!-- Nav -->
  <div class="flex flex-1 flex-col gap-0.5 px-2">
    {#each navItems as item}
      <button
        onclick={() => onNavigate(item.id)}
        title={item.label}
        class="nav-item flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left"
        class:active={activePage === item.id}
      >
        <span class="text-xs">{item.icon}</span>
        <span class="text-xs font-medium">{item.label}</span>
        {#if activePage === item.id}
          <span class="dot ml-auto h-1.5 w-1.5 rounded-full"></span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Footer -->
  <div class="px-4 pb-1">
    <p class="text-[10px]" style="color:var(--text-muted);">v0.1.0</p>
  </div>
</nav>

<style>
  .sidebar { background: var(--bg-surface); border-color: var(--border-subtle); }
  .logo { background: var(--accent); }

  .nav-item {
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }
  .nav-item:hover { background: var(--bg-hover); color: var(--text-base); }
  .nav-item.active { background: var(--accent-subtle); color: var(--accent); }
  .nav-item.active:hover { background: var(--accent-subtle); }

  .dot { background: var(--accent); }
</style>
