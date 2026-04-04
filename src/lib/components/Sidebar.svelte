<script lang="ts">
  import type { PageId } from "../types";

  interface Props {
    activePage: PageId;
    onNavigate: (page: PageId) => void;
  }

  let { activePage, onNavigate }: Props = $props();

  const navItems: { id: PageId; label: string; icon: string }[] = [
    { id: "launcher", label: "启动器", icon: "▶" },
    { id: "models", label: "模型", icon: "◆" },
    { id: "settings", label: "设置", icon: "⚙" },
  ];
</script>

<nav class="flex h-full w-14 flex-col items-center border-r border-[var(--border-color)] bg-[var(--bg-secondary)] py-3">
  <div class="mb-4 text-lg font-bold text-[var(--accent)]">L</div>

  <div class="flex flex-1 flex-col gap-1">
    {#each navItems as item}
      <button
        class="flex h-10 w-10 items-center justify-center rounded-lg text-sm transition-colors
          {activePage === item.id
          ? 'bg-[var(--accent)] text-white'
          : 'text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]'}"
        title={item.label}
        onclick={() => onNavigate(item.id)}
      >
        {item.icon}
      </button>
    {/each}
  </div>
</nav>
