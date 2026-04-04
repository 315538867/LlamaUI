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

  // 量化类型对应颜色
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
  <div
    class="flex shrink-0 items-center justify-between border-b px-4 py-3"
    style="border-color:var(--border-subtle); background:var(--bg-surface);"
  >
    <div>
      <h2 class="text-sm font-semibold" style="color:var(--text-base);">模型库</h2>
      <p class="text-xs" style="color:var(--text-muted);">管理本地 GGUF 模型文件</p>
    </div>
    <div class="flex items-center gap-2">
      <!-- 视图切换 -->
      <div class="flex rounded-md border overflow-hidden" style="border-color:var(--border-subtle);">
        <button
          onclick={() => (viewMode = "list")}
          class="px-2.5 py-1 text-[11px] transition-colors"
          style={viewMode === "list"
            ? "background:var(--bg-overlay); color:var(--text-base);"
            : "background:var(--bg-surface); color:var(--text-muted);"}
        >列表</button>
        <button
          onclick={() => (viewMode = "grid")}
          class="px-2.5 py-1 text-[11px] transition-colors border-l"
          style={viewMode === "grid"
            ? "background:var(--bg-overlay); color:var(--text-base); border-color:var(--border-subtle);"
            : "background:var(--bg-surface); color:var(--text-muted); border-color:var(--border-subtle);"}
        >网格</button>
      </div>
      <button
        onclick={() => modelStore.refresh()}
        class="rounded-md border px-3 py-1 text-[11px] transition-colors"
        style="border-color:var(--border-subtle); color:var(--text-secondary); background:var(--bg-surface);"
        onmouseenter={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-hover)")}
        onmouseleave={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-surface)")}
      >
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
      class="w-full rounded-md border px-3 py-2 text-xs"
      style="background:var(--bg-surface); border-color:var(--border-subtle); color:var(--text-base);"
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
          <div
            class="flex items-center justify-between rounded-lg border px-3 py-2.5 transition-colors"
            style="background:var(--bg-surface); border-color:var(--border-subtle);"
            onmouseenter={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-hover)")}
            onmouseleave={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-surface)")}
          >
            <div class="min-w-0 flex-1">
              <div class="truncate text-xs font-medium" style="color:var(--text-base);">{model.name}</div>
              <div class="truncate text-[11px] mt-0.5" style="color:var(--text-muted);">{model.path}</div>
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
          <div
            class="rounded-lg border p-3 transition-colors"
            style="background:var(--bg-surface); border-color:var(--border-subtle);"
            onmouseenter={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-hover)")}
            onmouseleave={(e) => ((e.currentTarget as HTMLElement).style.background = "var(--bg-surface)")}
          >
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
  <div
    class="shrink-0 border-t px-4 py-2 text-[11px]"
    style="border-color:var(--border-subtle); color:var(--text-muted);"
  >
    {filteredModels.length} 个模型
    {#if configStore.config.model_dirs.length === 0}
      · <span style="color:var(--warning);">未配置模型目录</span>
    {/if}
  </div>
</div>
