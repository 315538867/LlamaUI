<script lang="ts">
  import { getModelStore } from "../stores/models.svelte";
  import { getConfigStore } from "../stores/config.svelte";

  const modelStore = getModelStore();
  const configStore = getConfigStore();

  let searchQuery = $state("");
  let viewMode = $state<"grid" | "list">("list");

  const filteredModels = $derived(
    modelStore.models.filter((m) =>
      searchQuery
        ? m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (m.quantization?.toLowerCase().includes(searchQuery.toLowerCase()) ?? false)
        : true
    )
  );

  $effect(() => {
    if (configStore.loaded && modelStore.models.length === 0) {
      modelStore.refresh();
    }
  });
</script>

<div class="flex h-full flex-col gap-3 p-4">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold">模型管理</h2>
    <div class="flex items-center gap-2">
      <button
        class="rounded-lg border border-[var(--border-color)] px-3 py-1 text-xs text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-hover)]"
        onclick={() => modelStore.refresh()}
      >
        刷新
      </button>
      <button
        class="rounded px-2 py-1 text-xs transition-colors {viewMode === 'list'
          ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]'
          : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'}"
        onclick={() => (viewMode = "list")}
      >
        列表
      </button>
      <button
        class="rounded px-2 py-1 text-xs transition-colors {viewMode === 'grid'
          ? 'bg-[var(--bg-tertiary)] text-[var(--text-primary)]'
          : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'}"
        onclick={() => (viewMode = "grid")}
      >
        网格
      </button>
    </div>
  </div>

  <!-- Search -->
  <input
    type="text"
    bind:value={searchQuery}
    placeholder="搜索模型名称或量化类型..."
    class="rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] focus:border-[var(--accent)]"
  />

  <!-- Model List -->
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if modelStore.loading}
      <div class="flex items-center justify-center py-12 text-[var(--text-muted)]">
        扫描中...
      </div>
    {:else if modelStore.error}
      <div class="rounded-lg bg-[var(--bg-secondary)] p-4 text-center text-sm text-[var(--danger)]">
        {modelStore.error}
      </div>
    {:else if filteredModels.length === 0}
      <div class="flex flex-col items-center justify-center gap-2 py-12 text-[var(--text-muted)]">
        <p>未找到模型文件</p>
        <p class="text-xs">请在设置中配置模型目录</p>
      </div>
    {:else if viewMode === "list"}
      <div class="flex flex-col gap-1">
        {#each filteredModels as model}
          <div class="flex items-center justify-between rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] px-3 py-2 transition-colors hover:bg-[var(--bg-hover)]">
            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium text-[var(--text-primary)]">{model.name}</div>
              <div class="truncate text-xs text-[var(--text-muted)]">{model.path}</div>
            </div>
            <div class="flex items-center gap-3 pl-4 text-xs text-[var(--text-secondary)]">
              {#if model.quantization}
                <span class="rounded bg-[var(--bg-tertiary)] px-1.5 py-0.5">{model.quantization}</span>
              {/if}
              <span class="w-16 text-right">{model.size_display}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="grid grid-cols-2 gap-2 lg:grid-cols-3">
        {#each filteredModels as model}
          <div class="rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] p-3 transition-colors hover:bg-[var(--bg-hover)]">
            <div class="truncate text-sm font-medium text-[var(--text-primary)]">{model.name}</div>
            <div class="mt-1 flex items-center gap-2 text-xs text-[var(--text-muted)]">
              <span>{model.size_display}</span>
              {#if model.quantization}
                <span class="rounded bg-[var(--bg-tertiary)] px-1.5 py-0.5 text-[var(--text-secondary)]">{model.quantization}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="text-xs text-[var(--text-muted)]">
    共 {filteredModels.length} 个模型
    {#if configStore.config.model_dirs.length === 0}
      · <span class="text-[var(--warning)]">未配置模型目录</span>
    {/if}
  </div>
</div>
