<script lang="ts">
  import { getModelStore } from "../stores/models.svelte";
  import { getConfigStore } from "../stores/config.svelte";

  const modelStore = getModelStore();
  const configStore = getConfigStore();

  let searchQuery = $state("");
  let viewMode = $state<"list" | "grid">("list");
  let copiedPath = $state<string | null>(null);

  function copyName(model: { name: string; path: string }) {
    navigator.clipboard.writeText(model.name).then(() => {
      copiedPath = model.path;
      setTimeout(() => { copiedPath = null; }, 1500);
    });
  }

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

  function quantBg(q: string | null | undefined) {
    if (!q) return "var(--bg-overlay)";
    if (q.includes("Q8")) return "rgba(34,197,94,0.12)";
    if (q.includes("Q6") || q.includes("Q5")) return "rgba(59,130,246,0.12)";
    if (q.includes("Q4")) return "rgba(245,158,11,0.12)";
    if (q.includes("Q2") || q.includes("Q3")) return "rgba(239,68,68,0.12)";
    return "var(--bg-overlay)";
  }
  function quantFg(q: string | null | undefined) {
    if (!q) return "var(--text-muted)";
    if (q.includes("Q8")) return "var(--success)";
    if (q.includes("Q6") || q.includes("Q5")) return "var(--accent)";
    if (q.includes("Q4")) return "var(--warning)";
    if (q.includes("Q2") || q.includes("Q3")) return "var(--danger)";
    return "var(--text-muted)";
  }
</script>

<div class="root">

  <!-- 顶部栏 -->
  <div class="topbar">
    <div>
      <div class="topbar-title">模型库</div>
      <div class="topbar-sub">管理本地 GGUF 模型文件</div>
    </div>
    <div class="topbar-actions">
      <div class="view-toggle">
        <button class:active={viewMode === "list"} onclick={() => (viewMode = "list")}>列表</button>
        <button class:active={viewMode === "grid"} onclick={() => (viewMode = "grid")}>网格</button>
      </div>
      <button class="btn-ghost" onclick={() => modelStore.refresh()}>刷新</button>
    </div>
  </div>

  <!-- 搜索栏 -->
  <div class="search-bar">
    <input
      class="search-input"
      type="text"
      bind:value={searchQuery}
      placeholder="搜索模型名称或量化类型..."
    />
  </div>

  <!-- 内容区 -->
  <div class="content">
    {#if modelStore.loading}
      <div class="empty-state">
        <span class="empty-icon">⟳</span>
        <span>扫描中...</span>
      </div>
    {:else if modelStore.error}
      <div class="error-box">{modelStore.error}</div>
    {:else if filteredModels.length === 0}
      <div class="empty-state">
        <span class="empty-icon">◈</span>
        <p>未找到模型文件</p>
        <p class="empty-hint">请在设置中配置模型目录</p>
      </div>
    {:else if viewMode === "list"}
      <div class="list">
        {#each filteredModels as model}
          <div class="list-row">
            <div class="row-info">
              <button class="row-name-btn" onclick={() => copyName(model)} title="点击复制模型名">
                <span class="row-name">{model.name}</span>
                <span class="copy-hint">{copiedPath === model.path ? "已复制 ✓" : "复制"}</span>
              </button>
              <div class="row-path">{model.path}</div>
            </div>
            <div class="row-meta">
              {#if model.quantization}
                <span class="quant-badge" style="background:{quantBg(model.quantization)};color:{quantFg(model.quantization)};">
                  {model.quantization}
                </span>
              {/if}
              <span class="row-size">{model.size_display}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="grid">
        {#each filteredModels as model}
          <div class="grid-card">
            <button class="row-name-btn" onclick={() => copyName(model)} title="点击复制模型名">
              <span class="card-name">{model.name}</span>
              <span class="copy-hint">{copiedPath === model.path ? "✓" : "复制"}</span>
            </button>
            <div class="card-footer">
              <span class="row-size">{model.size_display}</span>
              {#if model.quantization}
                <span class="quant-badge" style="background:{quantBg(model.quantization)};color:{quantFg(model.quantization)};">
                  {model.quantization}
                </span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 底部状态栏 -->
  <div class="footer">
    <span>{filteredModels.length} 个模型</span>
    {#if searchQuery}
      <span class="footer-sep">·</span>
      <span>已过滤</span>
    {/if}
    {#if configStore.config.model_dirs.length === 0}
      <span class="footer-sep">·</span>
      <span style="color:var(--warning);">未配置模型目录</span>
    {/if}
  </div>
</div>

<style>
.root {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-base);
  overflow: hidden;
}

/* ─ Topbar ─ */
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}
.topbar-title { font-size: 13px; font-weight: 600; color: var(--text-base); line-height: 1.2; }
.topbar-sub { font-size: 11px; color: var(--text-muted); margin-top: 1px; }
.topbar-actions { display: flex; align-items: center; gap: 6px; }

/* ─ View toggle ─ */
.view-toggle {
  display: flex;
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  overflow: hidden;
}
.view-toggle button {
  padding: 4px 10px;
  font-size: 11px;
  background: var(--bg-surface);
  color: var(--text-muted);
  border: none;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.view-toggle button:not(:last-child) { border-right: 1px solid var(--border-subtle); }
.view-toggle button.active { background: var(--bg-overlay); color: var(--text-base); }
.view-toggle button:not(.active):hover { background: var(--bg-hover); }

/* ─ Ghost button ─ */
.btn-ghost {
  padding: 4px 10px;
  font-size: 11px;
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s;
}
.btn-ghost:hover { background: var(--bg-hover); }

/* ─ Search ─ */
.search-bar {
  padding: 10px 16px 6px;
  flex-shrink: 0;
}
.search-input {
  width: 100%;
  height: 28px;
  padding: 0 10px;
  font-size: 12px;
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  transition: border-color 0.12s;
}
.search-input::placeholder { color: var(--text-muted); }
.search-input:focus { border-color: var(--accent); }

/* ─ Content ─ */
.content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 16px;
}

/* ─ Empty / error ─ */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 64px 0;
  font-size: 12px;
  color: var(--text-muted);
}
.empty-icon { font-size: 24px; }
.empty-hint { font-size: 11px; }
.error-box {
  padding: 12px;
  font-size: 11px;
  color: var(--danger);
  background: rgba(239,68,68,0.08);
  border: 1px solid rgba(239,68,68,0.2);
  border-radius: 4px;
  text-align: center;
}

/* ─ Copy button ─ */
.row-name-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  text-align: left;
  min-width: 0;
  width: 100%;
}
.copy-hint {
  font-size: 10px;
  color: var(--accent);
  opacity: 0;
  flex-shrink: 0;
  transition: opacity 0.12s;
  white-space: nowrap;
}
.list-row:hover .copy-hint,
.grid-card:hover .copy-hint { opacity: 1; }

/* ─ List view ─ */
.list { display: flex; flex-direction: column; gap: 2px; }
.list-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 10px;
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  transition: background 0.12s;
}
.list-row:hover { background: var(--bg-hover); }
.row-info { flex: 1; min-width: 0; }
.row-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
  min-width: 0;
}
.row-path {
  font-size: 10px;
  color: var(--text-muted);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.row-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  padding-left: 12px;
}
.row-size {
  font-size: 11px;
  color: var(--text-muted);
  min-width: 48px;
  text-align: right;
}

/* ─ Grid view ─ */
.grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 6px;
}
@media (min-width: 1024px) { .grid { grid-template-columns: repeat(3, 1fr); } }
.grid-card {
  padding: 10px;
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  transition: background 0.12s;
}
.grid-card:hover { background: var(--bg-hover); }
.card-name {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
  min-width: 0;
}
.card-footer { display: flex; align-items: center; gap: 6px; margin-top: 8px; }

/* ─ Quant badge ─ */
.quant-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 5px;
  border-radius: 3px;
}

/* ─ Footer ─ */
.footer {
  flex-shrink: 0;
  padding: 6px 16px;
  font-size: 11px;
  color: var(--text-muted);
  border-top: 1px solid var(--border-subtle);
  display: flex;
  align-items: center;
  gap: 6px;
}
.footer-sep { opacity: 0.4; }
</style>
