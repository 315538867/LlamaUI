<script lang="ts">
  import "./app.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import Launcher from "./lib/components/Launcher.svelte";
  import ModelBrowser from "./lib/components/ModelBrowser.svelte";
  import ConfigEditor from "./lib/components/ConfigEditor.svelte";
  import { getConfigStore } from "./lib/stores/config.svelte";
  import type { PageId } from "./lib/types";

  const configStore = getConfigStore();

  let activePage = $state<PageId>("launcher");

  // Load config on mount
  $effect(() => {
    configStore.load();
  });
</script>

<div class="flex h-full w-full flex-col">
  <div class="flex min-h-0 flex-1">
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
