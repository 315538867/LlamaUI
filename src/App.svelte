<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import "./app.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import ModelLibrary from "./lib/components/ModelLibrary.svelte";
  import ModelBrowser from "./lib/components/ModelBrowser.svelte";
  import ConfigEditor from "./lib/components/ConfigEditor.svelte";
  import { getConfigStore } from "./lib/stores/config.svelte";
  import { getInstanceStore } from "./lib/stores/process.svelte";
  import type { PageId } from "./lib/types";

  const configStore = getConfigStore();
  const instanceStore = getInstanceStore();

  let activePage = $state<PageId>("instances");

  onMount(() => { configStore.load(); });
  onDestroy(() => { instanceStore.destroy(); });
</script>

<div class="flex h-full w-full flex-col" style="background:var(--bg-base);">
  <div class="flex min-h-0 flex-1 overflow-hidden">
    <Sidebar {activePage} onNavigate={(page) => (activePage = page)} />

    <main class="min-h-0 flex-1 overflow-hidden">
      {#if activePage === "instances"}
        <ModelLibrary />
      {:else if activePage === "models"}
        <ModelBrowser />
      {:else if activePage === "settings"}
        <ConfigEditor />
      {/if}
    </main>
  </div>

  <StatusBar />
</div>
