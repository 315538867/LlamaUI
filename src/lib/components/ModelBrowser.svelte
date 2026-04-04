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
    if (configStore.loaded && modelStore.models.length === 0) modelStore.refresh();
  });

  function quantColor(q: string | undefined): string {
    if (!q) return "var(--bg-overlay)";
    if (q.includes("Q8")) return "rgba(34,197,94,0.15)";
    if (q.includes("Q6") || q.includes("Q5")) return "rgba(59,130,246,0.15)";
    if (q.includes("Q4")) return "rgba(245,158,11,0.15)";
    if (q.includes("Q2") || q.includes("Q3")) return "rgba(239,68,68,0.15)";
    return "var(--bg-overlay)";
  }
  function quantTextColor(q: string | undefined): string {
    if (!q) return "var(--text-muted)";
    if (q.includes("Q8")) return "var(--success)";
    if (q.includes("Q6") || q.includes("Q5")) return "var(--accent)";
    if (q.includes("Q4")) return "var(--warning)";
    if (q.includes("Q2") || q.includes("Q3")) return "var(--danger)";
    return "var(--text-muted)";
  }
</script>

<div class="flex h-full flex-col" style="background:var(--bg-base);">

  <!-- 顶部栏 -->
  <div class="header flex shrink-0 items-center justify-between border-b px-4 py-3">
    <div>
      <h2 class="text-sm font-semibold" style="color:var(--text-base);">模型库</h2>
      <p class="text-xs" style="color:var(--text-muted);">管理本地 GGUF 模型文件</p>
    </div>
    <div class="flex items-center gap-2">
      <div class="view-toggle flex overflow-hidden rounded-md border">
        <button
          onclick={() => (viewMode = "list")}
          class="px-2.5 py-1 text-[11px] transition-colors"
          class:active={viewMode === "list"}
        >列表</button>
        <button
          onclick={() => (viewMode = "grid")}
          class="border-l px-2.5 py-1 text-[11px] transition-colors"
          class:active={viewMode === "grid"}
        >网格</button>
      </div>
      <button class="btn-ghost rounded-md border px-3 py-1 text-[11px]" onclick={() => modelStore.refresh()}>
        刷新
      </button>
    </div>
  </div>

  <!-- 搜索 -->
  <div class="shrink-0 px-4 pt-3">
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="搜索模型名称或量化类型..."
      class="search-input w-full rounded-md border px-3 py-2 text-xs"
    />
  </div>

  <!-- 列表 -->
  <div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
    {#if modelStore.loading}
      <div class="flex items-center justify-center py-16" style="color:var(--text-muted);">
        <span class="text-sm">扫描中...</span>
      </div>
    {:else if modelStore.error}
      <div class="rounded-lg p-4 text-center text-xs" style="background:var(--danger-subtle); color:var(--danger);">
        {modelStore.error}
      </div>
    {:else if filteredModels.length === 0}
      <div class="flex flex-col items-center justify-center gap-2 py-16" style="color:var(--text-muted);">
        <span class="text-2xl">◈</span>
        <p class="text-sm">未找到模型文件</p>
        <p class="text-xs">请在设置中配置模型目录</p>
      </div>
    {:else if viewMode === "list"}
      <div class="flex flex-col gap-1">
        {#each filteredModels as model}
          <div class="model-row flex items-center justify-between rounded-lg border px-3 py-2.5">
            <div class="min-w-0 flex-1">
              <div class="truncate text-xs font-medium" style="color:var(--text-base);">{model.name}</div>
              <div class="mt-0.5 truncate text-[11px]" style="color:var(--text-muted);">{model.path}</div>
            </div>
            <div class="flex shrink-0 items-center gap-2 pl-4">
              {#if model.quantization}
                <span
                  class="rounded px-1.5 py-0.5 text-[10px] font-medium"
                  style="background:{quantColor(model.quantization)}; color:{quantTextColor(model.quantization)};"
                >{model.quantization}</span>
              {/if}
              <span class="w-14 text-right text-[11px]" style="color:var(--text-muted);">{model.size_display}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="grid grid-cols-2 gap-2 lg:grid-cols-3">
        {#each filteredModels as model}
          <div class="model-card rounded-lg border p-3">
            <div class="truncate text-xs font-medium" style="color:var(--text-base);">{model.name}</div>
            <div class="mt-2 flex items-center gap-1.5">
              <span class="text-[11px]" style="color:var(--text-muted);">{model.size_display}</span>
              {#if model.quantization}
                <span
                  class="rounded px-1.5 py-0.5 text-[10px] font-medium"
                  style="background:{quantColor(model.quantization)}; color:{quantTextColor(model.quantization)};"
                >{model.quantization}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 底部统计 -->
  <div class="shrink-0 border-t px-4 py-2 text-[11px]" style="border-color:var(--border-subtle); color:var(--text-muted);">
    {filteredModels.length} 个模型
    {#if configStore.config.model_dirs.length === 0}
      · <span style="color:var(--warning);">未配置模型目录</span>
    {/if}
  </div>
</div>

<style>
  .header { border-color: var(--border-subtle); background: var(--bg-surface); }

  .view-toggle { border-color: var(--border-subtle); }
  .view-toggle button { background: var(--bg-surface); color: var(--text-muted); }
  .view-toggle button.active { background: var(--bg-overlay); color: var(--text-base); }
  .view-toggle .border-l { border-color: var(--border-subtle); }

  .btn-ghost {
    border-color: var(--border-subtle);
    color: var(--text-secondary);
    background: var(--bg-surface);
  }
  .btn-ghost:hover { background: var(--bg-hover); }

  .search-input {
    background: var(--bg-surface);
    border-color: var(--border-subtle);
    color: var(--text-base);
  }
  .search-input::placeholder { color: var(--text-muted); }
  .search-input:focus { border-color: var(--accent); outline: none; }

  .model-row {
    background: var(--bg-surface);
    border-color: var(--border-subtle);
    transition: background 0.15s;
  }
  .model-row:hover { background: var(--bg-hover); }

  .model-card {
    background: var(--bg-surface);
    border-color: var(--border-subtle);
    transition: background 0.15s;
  }
  .model-card:hover { background: var(--bg-hover); }
</style>
