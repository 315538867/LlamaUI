<script lang="ts">
  import { getConfigStore } from "../stores/config.svelte";
  import { detectLlama, validateLlamaPath } from "../services/tauri-bridge";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { AppConfig, LlamaInstall } from "../types";

  const configStore = getConfigStore();

  let llamaPath = $state("");
  let detectedInstalls = $state<LlamaInstall[]>([]);
  let detecting = $state(false);
  let validationMsg = $state("");

  $effect(() => {
    if (configStore.loaded) llamaPath = configStore.config.llama_dir ?? "";
  });

  async function handleDetect() {
    detecting = true;
    try { detectedInstalls = await detectLlama(); }
    catch (e) { console.error(e); }
    finally { detecting = false; }
  }

  async function selectLlamaDir() {
    const result = await open({ directory: true, title: "选择 llama.cpp 目录" });
    if (result) { llamaPath = result as string; await saveLlamaPath(); }
  }

  async function saveLlamaPath() {
    try {
      await validateLlamaPath(llamaPath);
      validationMsg = "";
      await configStore.save({ ...configStore.config, llama_dir: llamaPath });
    } catch (e) { validationMsg = String(e); }
  }

  function useDetected(install: LlamaInstall) {
    llamaPath = install.path;
    saveLlamaPath();
  }

  async function addModelDir() {
    const result = await open({ directory: true, title: "选择模型目录" });
    if (result && typeof result === "string") {
      await configStore.save({
        ...configStore.config,
        model_dirs: [...configStore.config.model_dirs, result],
      });
    }
  }

  async function removeModelDir(dir: string) {
    await configStore.save({
      ...configStore.config,
      model_dirs: configStore.config.model_dirs.filter((d) => d !== dir),
    });
  }
</script>

<div class="flex h-full flex-col" style="background:var(--bg-base);">

  <!-- 顶部栏 -->
  <div class="page-header flex shrink-0 items-center border-b px-4 py-3">
    <div>
      <h2 class="text-sm font-semibold" style="color:var(--text-base);">设置</h2>
      <p class="text-xs" style="color:var(--text-muted);">配置 llama.cpp 路径与模型目录</p>
    </div>
  </div>

  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-3">

    <!-- llama.cpp 路径 -->
    <section class="card rounded-lg border p-4">
      <h3 class="section-title mb-3 text-xs font-semibold uppercase tracking-wide">llama.cpp 路径</h3>
      <div class="flex gap-2">
        <input
          type="text"
          bind:value={llamaPath}
          placeholder="llama.cpp 安装目录路径..."
          onblur={saveLlamaPath}
          class="field-input flex-1 rounded-md border px-2.5 py-1.5 text-xs"
        />
        <button class="btn-ghost rounded-md border px-3 py-1.5 text-xs" onclick={selectLlamaDir}>浏览</button>
        <button class="btn-ghost rounded-md border px-3 py-1.5 text-xs" onclick={handleDetect} disabled={detecting}>
          {detecting ? "检测中..." : "自动检测"}
        </button>
      </div>

      {#if validationMsg}
        <p class="mt-2 text-xs" style="color:var(--danger);">{validationMsg}</p>
      {/if}

      {#if detectedInstalls.length > 0}
        <div class="mt-3 space-y-1">
          <p class="text-[11px] mb-1.5" style="color:var(--text-muted);">检测到 {detectedInstalls.length} 个安装：</p>
          {#each detectedInstalls as install}
            <button onclick={() => useDetected(install)} class="install-btn flex w-full items-center justify-between rounded-md border px-2.5 py-2 text-left text-xs">
              <span class="truncate">{install.path}</span>
              <span class="ml-3 shrink-0 text-[11px]" style="color:var(--text-muted);">
                {[install.has_server ? "server" : "", install.has_cli ? "cli" : ""].filter(Boolean).join(" + ")}
              </span>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <!-- 模型目录 -->
    <section class="card rounded-lg border p-4">
      <div class="mb-3 flex items-center justify-between">
        <h3 class="section-title text-xs font-semibold uppercase tracking-wide">模型目录</h3>
        <button onclick={addModelDir} class="btn-primary rounded-md px-3 py-1 text-xs font-medium text-white">
          + 添加目录
        </button>
      </div>

      {#if configStore.config.model_dirs.length === 0}
        <p class="text-xs" style="color:var(--text-muted);">未配置模型目录，请添加包含 .gguf 文件的目录</p>
      {:else}
        <div class="space-y-1">
          {#each configStore.config.model_dirs as dir}
            <div class="dir-item flex items-center justify-between rounded-md border px-2.5 py-2">
              <span class="truncate text-xs" style="color:var(--text-base);">{dir}</span>
              <button onclick={() => removeModelDir(dir)} class="remove-btn ml-3 shrink-0 text-xs">移除</button>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- 默认参数 -->
    <section class="card rounded-lg border p-4">
      <h3 class="section-title mb-2 text-xs font-semibold uppercase tracking-wide">默认启动参数</h3>
      <p class="text-xs" style="color:var(--text-muted);">默认参数在启动器中自动填充，可在启动前覆盖</p>
    </section>
  </div>
</div>

<style>
  .page-header { border-color: var(--border-subtle); background: var(--bg-surface); }
  .card { background: var(--bg-surface); border-color: var(--border-subtle); }
  .section-title { color: var(--text-muted); }

  .field-input {
    background: var(--bg-elevated);
    border-color: var(--border-subtle);
    color: var(--text-base);
  }
  .field-input:focus { border-color: var(--accent); outline: none; }

  .btn-ghost {
    border-color: var(--border);
    color: var(--text-secondary);
    background: var(--bg-elevated);
    transition: background 0.15s;
  }
  .btn-ghost:hover { background: var(--bg-hover); }

  .btn-primary { background: var(--accent); transition: opacity 0.15s; }
  .btn-primary:hover { opacity: 0.85; }

  .install-btn {
    background: var(--bg-elevated);
    border-color: var(--border-subtle);
    color: var(--text-base);
    transition: background 0.15s;
  }
  .install-btn:hover { background: var(--bg-hover); }

  .dir-item { background: var(--bg-elevated); border-color: var(--border-subtle); }

  .remove-btn { color: var(--text-muted); transition: color 0.15s; }
  .remove-btn:hover { color: var(--danger); }
</style>
