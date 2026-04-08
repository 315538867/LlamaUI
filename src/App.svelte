<script lang="ts">
  import { onMount } from "svelte";
  import "./app.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import ModelLibrary from "./lib/components/ModelLibrary.svelte";
  import ModelBrowser from "./lib/components/ModelBrowser.svelte";
  import ConfigEditor from "./lib/components/ConfigEditor.svelte";
  import ProxyConfigPanel from "./lib/components/ProxyConfigPanel.svelte";
  import { getConfigStore } from "./lib/stores/config.svelte";
  import { getInstanceStore } from "./lib/stores/process.svelte";
  import type { PageId } from "./lib/types";

  const configStore = getConfigStore();
  const instanceStore = getInstanceStore();

  let activePage = $state<PageId>("instances");
  let dismissedError = $state(false);
  let globalError = $state<string | null>(null);

  onMount(() => {
    configStore.load();

    window.onerror = (_msg, _src, _line, _col, err) => {
      globalError = err?.message ?? String(_msg);
      return false;
    };
    window.onunhandledrejection = (ev) => {
      globalError = ev.reason instanceof Error ? ev.reason.message : String(ev.reason);
    };
    window.addEventListener("beforeunload", () => instanceStore.destroy());
  });
</script>

<div class="flex h-full w-full flex-col" style="background:var(--bg-base);">
  {#if configStore.loadError && !dismissedError}
    <div class="load-error-bar">
      配置加载失败：{configStore.loadError}
      <button onclick={() => { dismissedError = true; }}>✕</button>
    </div>
  {/if}
  {#if globalError}
    <div class="load-error-bar">
      运行时错误：{globalError}
      <button onclick={() => { globalError = null; }}>✕</button>
    </div>
  {/if}
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <Sidebar {activePage} onNavigate={(page) => (activePage = page)} />

    <main class="min-h-0 flex-1 overflow-hidden">
      {#if activePage === "instances"}
        <ModelLibrary />
      {:else if activePage === "models"}
        <ModelBrowser />
      {:else if activePage === "proxy"}
        <ProxyConfigPanel />
      {:else if activePage === "settings"}
        <ConfigEditor />
      {/if}
    </main>
  </div>

  <StatusBar />
</div>

<style>
.load-error-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px;
  font-size: 12px;
  color: var(--danger);
  background: rgba(239, 68, 68, 0.08);
  border-bottom: 1px solid rgba(239, 68, 68, 0.2);
  flex-shrink: 0;
}
.load-error-bar button {
  background: none;
  border: none;
  color: var(--danger);
  cursor: pointer;
  font-size: 13px;
  padding: 0 2px;
  opacity: 0.7;
}
.load-error-bar button:hover { opacity: 1; }
</style>
