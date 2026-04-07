<script lang="ts">
  import { getConfigStore } from "../stores/config.svelte";
  import { getProxyStore } from "../stores/proxy.svelte";
  import { restartProxy, getProxyStatus } from "../services/tauri-bridge";
  import LogTerminal from "./LogTerminal.svelte";

  const configStore = getConfigStore();
  const proxyStore = getProxyStore();

  let proxyPort = $state(configStore.config.proxy_port ?? 8080);
  let proxyCors = $state(configStore.config.proxy_cors ?? true);
  let proxyAllowExternal = $state(configStore.config.proxy_allow_external ?? false);
  let proxyApiKey = $state(configStore.config.proxy_api_key ?? "");
  let proxyResponsesMode = $state<'direct' | 'anthropic'>(configStore.config.proxy_responses_mode ?? 'direct');
  let proxyApplying = $state(false);
  let proxyMsg = $state("");
  let proxyMsgTimer: ReturnType<typeof setTimeout> | undefined;

  let proxyRunning = $state(false);
  let proxyRoutes = $state<{ name: string; port: number }[]>([]);

  $effect(() => {
    if (configStore.loaded) {
      proxyPort = configStore.config.proxy_port ?? 8080;
      proxyCors = configStore.config.proxy_cors ?? true;
      proxyAllowExternal = configStore.config.proxy_allow_external ?? false;
      proxyApiKey = configStore.config.proxy_api_key ?? "";
      proxyResponsesMode = configStore.config.proxy_responses_mode ?? 'direct';
    }
  });

  $effect(() => {
    refreshStatus();
    const timer = setInterval(refreshStatus, 3000);
    return () => clearInterval(timer);
  });

  async function refreshStatus() {
    try {
      const s = await getProxyStatus();
      proxyRunning = s.running;
      proxyRoutes = s.routes ?? [];
    } catch { /* ignore */ }
  }

  const proxyLogLines = $derived(
    proxyStore.logs.map((e) => ({
      stream: e.level === "error" ? "stderr" : "stdout",
      line: `[${e.level.toUpperCase()}] ${e.message}`,
      ts: e.timestamp,
    }))
  );

  async function applyProxySettings() {
    proxyApplying = true;
    proxyMsg = "";
    try {
      await configStore.save({
        ...configStore.config,
        proxy_port: proxyPort,
        proxy_cors: proxyCors,
        proxy_allow_external: proxyAllowExternal,
        proxy_api_key: proxyApiKey || null,
        proxy_responses_mode: proxyResponsesMode,
      });
      await restartProxy(proxyPort, proxyCors, proxyAllowExternal, proxyApiKey || null, proxyResponsesMode);
      clearTimeout(proxyMsgTimer);
      proxyMsg = "已应用 ✓";
      proxyMsgTimer = setTimeout(() => { proxyMsg = ""; }, 2000);
      await refreshStatus();
    } catch (e) {
      proxyMsg = String(e);
    } finally {
      proxyApplying = false;
    }
  }
</script>

<div class="proxy-panel">
  <!-- 运行状态 -->
  <div class="proxy-status-section">
    <div class="proxy-section-title">运行状态</div>
    <div class="status-row">
      <span class="status-dot" class:running={proxyRunning}></span>
      <span class="status-label">{proxyRunning ? `运行中 · :${proxyPort}` : "已停止"}</span>
    </div>
    {#if proxyRoutes.length > 0}
      <div class="routes-title">路由表</div>
      <div class="routes-list">
        {#each proxyRoutes as r}
          <div class="route-item">
            <span class="route-name">{r.name}</span>
            <span class="route-arrow">→</span>
            <span class="route-port">:{r.port}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="routes-empty">暂无活跃路由</div>
    {/if}
  </div>

  <!-- 代理配置 -->
  <div class="proxy-config-section">
    <div class="proxy-section-title">代理配置</div>
    <div class="proxy-fields">
      <div class="proxy-field-row">
        <label class="proxy-field-label" for="p-port">端口</label>
        <input id="p-port" class="field-input w-num" type="number" min="1" max="65535" bind:value={proxyPort} />
      </div>
      <div class="proxy-field-row">
        <label class="proxy-field-label" for="p-apikey">API Key</label>
        <input id="p-apikey" class="field-input flex-1" type="password" bind:value={proxyApiKey} placeholder="可选，保护代理入口" />
      </div>
      <div class="proxy-field-row">
        <span class="proxy-field-label">CORS</span>
        <label class="toggle">
          <input type="checkbox" bind:checked={proxyCors} />
          <span class="toggle-track"></span>
        </label>
      </div>
      <div class="proxy-field-row">
        <span class="proxy-field-label">局域网访问</span>
        <label class="toggle">
          <input type="checkbox" bind:checked={proxyAllowExternal} />
          <span class="toggle-track"></span>
        </label>
      </div>
      <div class="proxy-field-row">
        <span class="proxy-field-label">转换模式</span>
        <div class="mode-toggle">
          <button
            class="mode-btn"
            class:active={proxyResponsesMode === 'direct'}
            onclick={() => proxyResponsesMode = 'direct'}
          >直连</button>
          <button
            class="mode-btn"
            class:active={proxyResponsesMode === 'anthropic'}
            onclick={() => proxyResponsesMode = 'anthropic'}
          >Anthropic</button>
        </div>
      </div>
    </div>
    <div class="proxy-apply-row">
      <button class="action-btn btn-start" onclick={applyProxySettings} disabled={proxyApplying}>
        {proxyApplying ? "应用中..." : "应用"}
      </button>
      {#if proxyMsg}
        <span class="proxy-apply-msg" class:ok={proxyMsg.startsWith("已应用")}>{proxyMsg}</span>
      {/if}
    </div>
  </div>

  <!-- 代理日志 -->
  <div class="proxy-log-section">
    <div class="log-toolbar">
      <span class="log-label">代理日志</span>
      <button class="btn-ghost-sm" onclick={() => proxyStore.clearLogs()}>清空</button>
    </div>
    <LogTerminal logs={proxyLogLines} />
  </div>
</div>

<style>
.proxy-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.proxy-status-section {
  flex-shrink: 0;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border);
  flex-shrink: 0;
}
.status-dot.running { background: var(--success); }

.status-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.routes-title {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.routes-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.route-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  padding: 3px 6px;
  background: var(--bg-elevated);
  border-radius: 4px;
  border: 1px solid var(--border-subtle);
}

.route-name { color: var(--text-base); font-weight: 500; }
.route-arrow { color: var(--text-muted); }
.route-port { color: var(--accent); font-family: monospace; }

.routes-empty {
  font-size: 11px;
  color: var(--text-muted);
}

.proxy-config-section {
  flex-shrink: 0;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.proxy-section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.proxy-fields {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 10px;
}

.proxy-field-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 26px;
}

.proxy-field-label {
  font-size: 11px;
  color: var(--text-secondary);
  width: 80px;
  flex-shrink: 0;
}

.proxy-apply-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.proxy-apply-msg {
  font-size: 11px;
  color: var(--text-muted);
}
.proxy-apply-msg.ok { color: var(--success); }

.proxy-log-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.log-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.log-label {
  font-size: 11px;
  color: var(--text-muted);
}

.btn-ghost-sm {
  font-size: 10px;
  padding: 2px 6px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
}
.btn-ghost-sm:hover { background: var(--bg-hover); }

.field-input {
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
.field-input:focus { border-color: var(--accent); }
.field-input.flex-1 { flex: 1; }
.field-input.w-num { width: 80px; }

.toggle {
  display: flex;
  align-items: center;
  cursor: pointer;
  position: relative;
}
.toggle input { position: absolute; opacity: 0; width: 0; height: 0; }
.toggle-track {
  width: 32px;
  height: 18px;
  border-radius: 9px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  transition: background 0.15s, border-color 0.15s;
  position: relative;
}
.toggle-track::after {
  content: "";
  position: absolute;
  left: 2px;
  top: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: left 0.15s, background 0.15s;
}
.toggle input:checked + .toggle-track {
  background: rgba(59,130,246,0.25);
  border-color: var(--accent);
}
.toggle input:checked + .toggle-track::after {
  left: 16px;
  background: var(--accent);
}

.action-btn {
  font-size: 11px;
  font-weight: 500;
  height: 24px;
  padding: 0 12px;
  border-radius: 4px;
  border: none;
  cursor: pointer;
  transition: opacity 0.12s;
}
.action-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-start { background: var(--accent); color: #fff; }
.btn-start:not(:disabled):hover { opacity: 0.85; }

.mode-toggle {
  display: flex;
  gap: 4px;
}
.mode-btn {
  font-size: 11px;
  font-weight: 500;
  height: 22px;
  padding: 0 10px;
  border-radius: 4px;
  border: 1px solid var(--border-subtle);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.12s;
}
.mode-btn.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.mode-btn:not(.active):hover {
  border-color: var(--accent);
  color: var(--accent);
}
</style>
