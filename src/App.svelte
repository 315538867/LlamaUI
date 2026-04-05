<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import "./app.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import Launcher from "./lib/components/Launcher.svelte";
  import ModelBrowser from "./lib/components/ModelBrowser.svelte";
  import ConfigEditor from "./lib/components/ConfigEditor.svelte";
  import { getConfigStore } from "./lib/stores/config.svelte";
  import { getProcessStore } from "./lib/stores/process.svelte";
  import type { PageId } from "./lib/types";

  const configStore = getConfigStore();

  let activePage = $state<PageId>("launcher");

  onMount(() => { configStore.load(); });
  onDestroy(() => { getProcessStore().destroy(); });
</script>

<div class="flex h-full w-full flex-col" style="background:var(--bg-base);">
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <Sidebar {activePage} onNavigate={(page) => (activePage = page)} />

    <main class="min-h-0 flex-1 overflow-hidden">
      {#if activePage === "launcher"}
        <Launcher />
      {:else if activePage === "models"}
        <ModelBrowser />
      {:else if activePage === "settings"}
        <ConfigEditor />
      {/if}
    </main>
  </div>

  <StatusBar />
</div>
