<script lang="ts">
  import { getConfigStore } from "../stores/config.svelte";
  import { detectLlama, validateLlamaPath } from "../services/tauri-bridge";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { AppConfig, LlamaInstall } from "../types";

  const configStore = getConfigStore();

  let llamaPath = $state("");
  let detectedInstalls = $state<LlamaInstall[]>([]);
  let detecting = $state(false);
  let newModelDir = $state("");
  let validationMsg = $state("");

  $effect(() => {
    if (configStore.loaded) {
      llamaPath = configStore.config.llama_dir ?? "";
    }
  });

  async function handleDetect() {
    detecting = true;
    try {
      detectedInstalls = await detectLlama();
    } catch (e) {
      console.error(e);
    } finally {
      detecting = false;
    }
  }

  async function selectLlamaDir() {
    const result = await open({ directory: true, title: "选择 llama.cpp 目录" });
    if (result) {
      llamaPath = result as string;
      await saveLlamaPath();
    }
  }

  async function saveLlamaPath() {
    try {
      await validateLlamaPath(llamaPath);
      validationMsg = "";
      const newConfig: AppConfig = { ...configStore.config, llama_dir: llamaPath };
      await configStore.save(newConfig);
    } catch (e) {
      validationMsg = String(e);
    }
  }

  function useDetected(install: LlamaInstall) {
    llamaPath = install.path;
    saveLlamaPath();
  }

  async function addModelDir() {
    const result = await open({ directory: true, title: "选择模型目录" });
    if (result && typeof result === "string") {
      const dirs = [...configStore.config.model_dirs, result];
      const newConfig: AppConfig = { ...configStore.config, model_dirs: dirs };
      await configStore.save(newConfig);
    }
  }

  async function removeModelDir(dir: string) {
    const dirs = configStore.config.model_dirs.filter((d) => d !== dir);
    const newConfig: AppConfig = { ...configStore.config, model_dirs: dirs };
    await configStore.save(newConfig);
  }
</script>

<div class="flex h-full flex-col gap-4 overflow-y-auto p-4">
  <h2 class="text-lg font-semibold">设置</h2>

  <!-- llama.cpp Path -->
  <section class="rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] p-4">
    <h3 class="mb-3 text-sm font-medium">llama.cpp 路径</h3>
    <div class="flex gap-2">
      <input
        type="text"
        bind:value={llamaPath}
        placeholder="llama.cpp 安装目录路径..."
        class="flex-1 rounded-md border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
        onblur={saveLlamaPath}
      />
      <button
        class="rounded-md border border-[var(--border-color)] px-3 py-1.5 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
        onclick={selectLlamaDir}
      >
        浏览
      </button>
      <button
        class="rounded-md border border-[var(--border-color)] px-3 py-1.5 text-xs text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
        onclick={handleDetect}
        disabled={detecting}
      >
        {detecting ? "检测中..." : "自动检测"}
      </button>
    </div>
    {#if validationMsg}
      <p class="mt-1 text-xs text-[var(--danger)]">{validationMsg}</p>
    {/if}
    {#if detectedInstalls.length > 0}
      <div class="mt-2 flex flex-col gap-1">
        <p class="text-xs text-[var(--text-muted)]">检测到 {detectedInstalls.length} 个安装:</p>
        {#each detectedInstalls as install}
          <button
            class="flex items-center justify-between rounded border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-[var(--bg-hover)]"
            onclick={() => useDetected(install)}
          >
            <span class="text-[var(--text-primary)]">{install.path}</span>
            <span class="text-[var(--text-muted)]">
              {install.has_server ? "server" : ""}{install.has_server && install.has_cli ? " + " : ""}{install.has_cli ? "cli" : ""}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Model Directories -->
  <section class="rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] p-4">
    <div class="mb-3 flex items-center justify-between">
      <h3 class="text-sm font-medium">模型目录</h3>
      <button
        class="rounded-md bg-[var(--accent)] px-3 py-1 text-xs text-white hover:bg-[var(--accent-hover)]"
        onclick={addModelDir}
      >
        添加目录
      </button>
    </div>
    {#if configStore.config.model_dirs.length === 0}
      <p class="text-xs text-[var(--text-muted)]">未配置模型目录，请添加包含 .gguf 文件的目录</p>
    {:else}
      <div class="flex flex-col gap-1">
        {#each configStore.config.model_dirs as dir}
          <div class="flex items-center justify-between rounded border border-[var(--border-color)] bg-[var(--bg-tertiary)] px-2.5 py-1.5">
            <span class="truncate text-xs text-[var(--text-primary)]">{dir}</span>
            <button
              class="ml-2 text-xs text-[var(--text-muted)] hover:text-[var(--danger)]"
              onclick={() => removeModelDir(dir)}
            >
              移除
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- Default Parameters -->
  <section class="rounded-lg border border-[var(--border-color)] bg-[var(--bg-secondary)] p-4">
    <h3 class="mb-3 text-sm font-medium">默认启动参数</h3>
    <p class="text-xs text-[var(--text-muted)]">默认参数在启动器中自动填充，可在启动前覆盖</p>
  </section>
</div>
