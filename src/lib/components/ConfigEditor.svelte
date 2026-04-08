<script lang="ts">
  import { getConfigStore } from "../stores/config.svelte";
  import { detectLlama, validateLlamaPath } from "../services/tauri-bridge";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { LlamaInstall } from "../types";
  import { logger } from "../utils/logger";

  const configStore = getConfigStore();

  let llamaPath = $state("");
  let detectedInstalls = $state<LlamaInstall[]>([]);
  let detecting = $state(false);
  let validationMsg = $state("");
  let saveError = $state("");
  let copied = $state<string | null>(null);
  let saveErrorTimer: ReturnType<typeof setTimeout> | undefined;

  function showSaveError(e: unknown) {
    clearTimeout(saveErrorTimer);
    saveError = String(e);
    saveErrorTimer = setTimeout(() => { saveError = ""; }, 3000);
  }

  $effect(() => {
    if (configStore.loaded) {
      llamaPath = configStore.config.llama_dir ?? "";
    }
  });

  const serverApiKey = $derived(configStore.config.proxy_api_key ?? "");
  const baseUrl = $derived(`http://127.0.0.1:${configStore.config.proxy_port ?? 8080}`);

  async function handleDetect() {
    detecting = true;
    detectedInstalls = [];
    try { detectedInstalls = await detectLlama(); }
    catch (e) { logger.error("detectLlama failed:", e); }
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
    } catch (e) {
      validationMsg = String(e);
      return;
    }
    try {
      await configStore.save({ ...configStore.config, llama_dir: llamaPath });
    } catch (e) { showSaveError(e); }
  }

  function useDetected(install: LlamaInstall) {
    llamaPath = install.path;
    saveLlamaPath();
  }

  async function addModelDir() {
    const result = await open({ directory: true, title: "选择模型目录" });
    if (result && typeof result === "string") {
      try {
        await configStore.save({
          ...configStore.config,
          model_dirs: [...configStore.config.model_dirs, result],
        });
      } catch (e) { showSaveError(e); }
    }
  }

  async function removeModelDir(dir: string) {
    try {
      await configStore.save({
        ...configStore.config,
        model_dirs: configStore.config.model_dirs.filter((d) => d !== dir),
      });
    } catch (e) { showSaveError(e); }
  }

  function copyText(text: string, key: string) {
    navigator.clipboard.writeText(text).then(() => {
      copied = key;
      setTimeout(() => { copied = null; }, 1500);
    });
  }

  // Codex CLI 环境变量
  const codexEnv = $derived(
    `export OPENAI_BASE_URL=${baseUrl}/v1\nexport OPENAI_API_KEY=${serverApiKey || "sk-no-key-required"}`
  );
  // Claude Code 环境变量
  const claudeCodeEnv = $derived(
    `export ANTHROPIC_BASE_URL=${baseUrl}/anthropic\nexport ANTHROPIC_API_KEY=${serverApiKey || "sk-no-key-required"}`
  );
</script>

<div class="root">

  <!-- 顶部栏 -->
  <div class="topbar">
    <div>
      <div class="topbar-title">设置</div>
      <div class="topbar-sub">配置 llama.cpp 路径、模型目录与客户端接入</div>
    </div>
  </div>

  <div class="body">

    <!-- ─ llama.cpp 路径 ─ -->
    <div class="section">
      <div class="section-title">llama.cpp 路径</div>
      <div class="row-input">
        <input
          class="input flex-1"
          type="text"
          bind:value={llamaPath}
          placeholder="llama.cpp 安装目录路径..."
          onblur={saveLlamaPath}
        />
        <button class="btn-ghost" onclick={selectLlamaDir}>浏览</button>
        <button class="btn-ghost" onclick={handleDetect} disabled={detecting}>
          {detecting ? "检测中..." : "自动检测"}
        </button>
      </div>

      {#if validationMsg}
        <div class="error-bar">{validationMsg}</div>
      {/if}

      {#if detectedInstalls.length > 0}
        <div class="detect-list">
          <div class="detect-label">检测到 {detectedInstalls.length} 个安装</div>
          {#each detectedInstalls as install}
            <button onclick={() => useDetected(install)} class="detect-item">
              <span class="detect-path">{install.path}</span>
              <span class="detect-caps">
                {[install.has_server ? "server" : "", install.has_cli ? "cli" : ""].filter(Boolean).join(" + ")}
              </span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    {#if saveError}
      <div class="error-bar" style="margin:0 0 4px">{saveError}</div>
    {/if}

    <!-- ─ 模型目录 ─ -->
    <div class="section">
      <div class="section-header">
        <span class="section-title">模型目录</span>
        <button class="btn-primary" onclick={addModelDir}>+ 添加</button>
      </div>

      {#if configStore.config.model_dirs.length === 0}
        <div class="empty-hint">未配置模型目录，请添加包含 .gguf 文件的文件夹</div>
      {:else}
        <div class="dir-list">
          {#each configStore.config.model_dirs as dir}
            <div class="dir-item">
              <span class="dir-path">{dir}</span>
              <button class="remove-btn" onclick={() => removeModelDir(dir)}>移除</button>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- ─ 客户端接入 ─ -->
    <div class="section">
      <div class="section-title">客户端接入</div>
      <div class="access-note">
        通过代理地址 <code>{baseUrl}</code> 访问。llama.cpp 在本地随机端口运行，代理负责转发与协议转换。如需局域网访问，请开启"允许局域网访问"，并将 <code>127.0.0.1</code> 替换为本机 IP。
      </div>

      <!-- Codex CLI -->
      <div class="client-card">
        <div class="client-header">
          <div class="client-name">Codex CLI</div>
          <div class="client-badge badge-openai">OpenAI 兼容</div>
          <button class="copy-btn" onclick={() => copyText(codexEnv, "codex")}>
            {copied === "codex" ? "已复制 ✓" : "复制"}
          </button>
        </div>
        <pre class="code-block">{codexEnv}</pre>
        <div class="client-apis">
          接口：<code>/v1/chat/completions</code> · <code>/v1/models</code> · <code>/v1/responses</code>
        </div>
      </div>

      <!-- Claude Code -->
      <div class="client-card">
        <div class="client-header">
          <div class="client-name">Claude Code</div>
          <div class="client-badge badge-anthropic">Anthropic 兼容</div>
          <button class="copy-btn" onclick={() => copyText(claudeCodeEnv, "claudeCode")}>
            {copied === "claudeCode" ? "已复制 ✓" : "复制"}
          </button>
        </div>
        <pre class="code-block">{claudeCodeEnv}</pre>
        <div class="client-apis">
          接口：<code>/anthropic/v1/messages</code>
        </div>
      </div>
    </div>

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
  padding: 10px 16px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}
.topbar-title { font-size: 13px; font-weight: 600; color: var(--text-base); line-height: 1.2; }
.topbar-sub { font-size: 11px; color: var(--text-muted); margin-top: 1px; }

/* ─ Body ─ */
.body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 16px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* ─ Section ─ */
.section {
  padding: 8px 0 12px;
  border-bottom: 1px solid var(--border-subtle);
}
.section:last-child { border-bottom: none; }
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 8px;
  display: block;
}
.section-header .section-title { margin-bottom: 0; }

/* ─ Input row ─ */
.row-input { display: flex; gap: 6px; align-items: center; }
.input {
  height: 26px;
  padding: 0 8px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  transition: border-color 0.12s;
  min-width: 0;
}
.input:focus { border-color: var(--accent); }
.flex-1 { flex: 1; }

/* ─ Buttons ─ */
.btn-ghost {
  height: 26px;
  padding: 0 10px;
  font-size: 11px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
  transition: background 0.12s;
}
.btn-ghost:hover { background: var(--bg-hover); }
.btn-ghost:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-primary {
  height: 24px;
  padding: 0 10px;
  font-size: 11px;
  font-weight: 500;
  background: var(--accent);
  color: #fff;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.12s;
}
.btn-primary:hover { opacity: 0.85; }

/* ─ Error ─ */
.error-bar {
  margin-top: 6px;
  padding: 6px 10px;
  font-size: 11px;
  color: var(--danger);
  background: rgba(239,68,68,0.08);
  border: 1px solid rgba(239,68,68,0.2);
  border-radius: 4px;
}

/* ─ Detected installs ─ */
.detect-list { margin-top: 8px; display: flex; flex-direction: column; gap: 3px; }
.detect-label { font-size: 10px; color: var(--text-muted); margin-bottom: 3px; }
.detect-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.12s;
  text-align: left;
}
.detect-item:hover { background: var(--bg-hover); }
.detect-path { font-size: 11px; color: var(--text-base); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.detect-caps { font-size: 10px; color: var(--text-muted); flex-shrink: 0; margin-left: 8px; }

/* ─ Dirs ─ */
.empty-hint { font-size: 11px; color: var(--text-muted); }
.dir-list { display: flex; flex-direction: column; gap: 3px; }
.dir-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
}
.dir-path { font-size: 11px; color: var(--text-base); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.remove-btn {
  font-size: 11px;
  color: var(--text-muted);
  background: none;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
  margin-left: 8px;
  transition: color 0.12s;
}
.remove-btn:hover { color: var(--danger); }

/* ─ Client access ─ */
.access-note {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 10px;
  line-height: 1.5;
}
.access-note code {
  font-family: monospace;
  font-size: 10px;
  background: var(--bg-overlay);
  padding: 1px 4px;
  border-radius: 2px;
  color: var(--text-secondary);
}

.client-card {
  background: var(--bg-surface);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  padding: 10px 12px;
  margin-bottom: 8px;
}
.client-card:last-child { margin-bottom: 0; }
.client-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.client-name { font-size: 12px; font-weight: 600; color: var(--text-base); }
.client-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 500;
}
.badge-openai { background: rgba(16,163,127,0.12); color: #10a37f; }
.badge-anthropic { background: rgba(205,154,109,0.15); color: #cd9a6d; }

.copy-btn {
  margin-left: auto;
  font-size: 11px;
  padding: 2px 8px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.12s;
}
.copy-btn:hover { background: var(--bg-hover); }

.code-block {
  font-family: monospace;
  font-size: 11px;
  line-height: 1.6;
  color: var(--text-base);
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  padding: 8px 10px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
.client-apis {
  margin-top: 6px;
  font-size: 10px;
  color: var(--text-muted);
  line-height: 1.6;
}
.client-apis code {
  font-family: monospace;
  background: var(--bg-overlay);
  padding: 1px 4px;
  border-radius: 2px;
  color: var(--text-secondary);
}
</style>
