<script lang="ts">
  import type { LaunchParams, InstanceInfo } from "../types";

  interface Props {
    editName: string;
    editModelPath: string;
    editMode: "server" | "cli";
    editParams: LaunchParams;
    isCreating: boolean;
    scanning: boolean;
    selectedInfo: InstanceInfo | null;
    onNameChange: (v: string) => void;
    onModelPathChange: (v: string) => void;
    onModeChange: (v: "server" | "cli") => void;
    onParamsChange: (p: LaunchParams) => void;
    onScan: () => void;
  }

  let {
    editName,
    editModelPath,
    editMode,
    editParams,
    isCreating,
    scanning,
    selectedInfo,
    onNameChange,
    onModelPathChange,
    onModeChange,
    onParamsChange,
    onScan,
  }: Props = $props();

  function setParam<K extends keyof LaunchParams>(key: K, value: LaunchParams[K]) {
    onParamsChange({ ...editParams, [key]: value });
  }

  function numInput(key: keyof LaunchParams, raw: string) {
    setParam(key, raw === "" ? null : parseInt(raw) as any);
  }
</script>

<div class="config-body">
  <!-- Instance name -->
  <div class="field-row">
    <label class="field-label" for="edit-name">实例名称</label>
    <input id="edit-name" class="field-input" type="text"
      value={editName}
      oninput={(e) => onNameChange((e.target as HTMLInputElement).value)}
      placeholder="my-model（作为 Codex model 字段）"
      disabled={!isCreating}
    />
  </div>
  <div class="field-hint">此名称将作为 Codex 请求中的 <code>model</code> 字段路由到此实例</div>

  <!-- Model path -->
  <div class="field-row">
    <label class="field-label" for="edit-model">模型文件</label>
    <div class="model-picker">
      <input id="edit-model" class="field-input flex-1" type="text"
        value={editModelPath}
        oninput={(e) => onModelPathChange((e.target as HTMLInputElement).value)}
        placeholder="/path/to/model.gguf"
        readonly={isCreating}
      />
      {#if !isCreating}
        <button class="btn-ghost" onclick={onScan} disabled={scanning}>
          {scanning ? "扫描中..." : "扫描"}
        </button>
      {/if}
    </div>
  </div>

  <!-- Mode -->
  <div class="field-row">
    <label class="field-label" for="edit-mode">运行模式</label>
    <select id="edit-mode" class="field-select"
      value={editMode}
      onchange={(e) => onModeChange((e.target as HTMLSelectElement).value as "server" | "cli")}
    >
      <option value="server">server（HTTP API）</option>
      <option value="cli">cli（交互式）</option>
    </select>
  </div>

  <div class="section-divider">参数</div>

  <div class="field-row">
    <label class="field-label" for="edit-gpu">GPU 层数</label>
    <input id="edit-gpu" class="field-input w-num" type="number"
      value={editParams.gpu_layers ?? ""}
      oninput={(e) => numInput("gpu_layers", (e.target as HTMLInputElement).value)}
      placeholder="99"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-ctx">上下文大小</label>
    <input id="edit-ctx" class="field-input w-num" type="number"
      value={editParams.ctx_size ?? ""}
      oninput={(e) => numInput("ctx_size", (e.target as HTMLInputElement).value)}
      placeholder="4096"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-parallel">并发槽数</label>
    <input id="edit-parallel" class="field-input w-num" type="number"
      value={editParams.parallel ?? ""}
      oninput={(e) => numInput("parallel", (e.target as HTMLInputElement).value)}
      placeholder="1"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-threads">CPU 线程</label>
    <input id="edit-threads" class="field-input w-num" type="number"
      value={editParams.threads ?? ""}
      oninput={(e) => numInput("threads", (e.target as HTMLInputElement).value)}
      placeholder="自动"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-batch">批处理大小</label>
    <input id="edit-batch" class="field-input w-num" type="number"
      value={editParams.batch_size ?? ""}
      oninput={(e) => numInput("batch_size", (e.target as HTMLInputElement).value)}
      placeholder="512"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-ubatch">解码批次</label>
    <input id="edit-ubatch" class="field-input w-num" type="number"
      value={editParams.ubatch_size ?? ""}
      oninput={(e) => numInput("ubatch_size", (e.target as HTMLInputElement).value)}
      placeholder="512"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-cache-k">KV Cache K</label>
    <select id="edit-cache-k" class="field-select" style="flex:1;"
      value={editParams.cache_type_k ?? "f16"}
      onchange={(e) => { const v = (e.target as HTMLSelectElement).value; setParam("cache_type_k", v === "f16" ? null : v); }}
    >
      <option value="f16">f16（默认）</option>
      <option value="q8_0">q8_0</option>
      <option value="q4_0">q4_0</option>
      <option value="q4_1">q4_1</option>
      <option value="iq4_nl">iq4_nl</option>
      <option value="q5_0">q5_0</option>
      <option value="q5_1">q5_1</option>
    </select>
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-cache-v">KV Cache V</label>
    <select id="edit-cache-v" class="field-select" style="flex:1;"
      value={editParams.cache_type_v ?? "f16"}
      onchange={(e) => { const v = (e.target as HTMLSelectElement).value; setParam("cache_type_v", v === "f16" ? null : v); }}
    >
      <option value="f16">f16（默认）</option>
      <option value="q8_0">q8_0</option>
      <option value="q4_0">q4_0</option>
      <option value="q4_1">q4_1</option>
      <option value="iq4_nl">iq4_nl</option>
      <option value="q5_0">q5_0</option>
      <option value="q5_1">q5_1</option>
    </select>
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-seed">随机种子</label>
    <input id="edit-seed" class="field-input w-num" type="number"
      value={editParams.seed ?? ""}
      oninput={(e) => numInput("seed", (e.target as HTMLInputElement).value)}
      placeholder="-1（随机）"
    />
  </div>

  {#if editMode === "server"}
    <div class="field-row system-prompt-row">
      <label class="field-label" for="edit-system-prompt" style="padding-top: 6px;">系统提示</label>
      <textarea id="edit-system-prompt" class="field-textarea"
        value={editParams.system_prompt ?? ""}
        oninput={(e) => setParam("system_prompt", (e.target as HTMLTextAreaElement).value || null)}
        placeholder="你是一个有帮助的 AI 助手"
      ></textarea>
    </div>
  {/if}

  <div class="field-row">
    <span class="field-label">Flash Attention</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.flash_attn ?? false}
        onchange={(e) => setParam("flash_attn", (e.target as HTMLInputElement).checked)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <span class="field-label">持续批处理</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.cont_batching ?? false}
        onchange={(e) => setParam("cont_batching", (e.target as HTMLInputElement).checked)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <span class="field-label">锁定内存</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.mlock ?? false}
        onchange={(e) => setParam("mlock", (e.target as HTMLInputElement).checked || null)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <span class="field-label">禁用 mmap</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.no_mmap ?? false}
        onchange={(e) => setParam("no_mmap", (e.target as HTMLInputElement).checked || null)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <span class="field-label">禁用 KV 卸载</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.no_kv_offload ?? false}
        onchange={(e) => setParam("no_kv_offload", (e.target as HTMLInputElement).checked || null)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <span class="field-label">禁用上下文移位</span>
    <label class="toggle">
      <input type="checkbox"
        checked={editParams.no_context_shift ?? false}
        onchange={(e) => setParam("no_context_shift", (e.target as HTMLInputElement).checked || null)}
      />
      <span class="toggle-track"></span>
    </label>
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-keep">保留头部Token数</label>
    <input id="edit-keep" class="field-input" type="number"
      value={editParams.keep ?? ""}
      oninput={(e) => numInput("keep", (e.target as HTMLInputElement).value)}
      placeholder="-1=全部 0=不保留"
      style="width:120px"
    />
  </div>

  <div class="field-row">
    <label class="field-label" for="edit-extra">额外参数</label>
    <input id="edit-extra" class="field-input flex-1" type="text"
      value={editParams.extra_args ?? ""}
      oninput={(e) => setParam("extra_args", (e.target as HTMLInputElement).value || null)}
      placeholder="如 --no-mmap --mlock"
    />
  </div>

  {#if selectedInfo?.status === "running"}
    <div class="running-info">
      <span>端口 <code>:{selectedInfo.port}</code></span>
      <span>PID <code>{selectedInfo.pid}</code></span>
    </div>
  {/if}
</div>

<style>
.config-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-divider {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--text-muted);
  border-top: 1px solid var(--border-subtle);
  padding-top: 8px;
  margin-top: 4px;
}

.field-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 28px;
}

.field-label {
  font-size: 11px;
  color: var(--text-secondary);
  width: 100px;
  flex-shrink: 0;
}

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

.field-textarea {
  padding: 6px 8px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  transition: border-color 0.12s;
  resize: none;
  flex: 1;
  font-family: inherit;
  line-height: 1.4;
}
.field-textarea:focus { border-color: var(--accent); }

.system-prompt-row {
  align-items: flex-start;
  height: 84px;
}

.field-select {
  height: 26px;
  padding: 0 6px;
  font-size: 12px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
  border-radius: 4px;
  color: var(--text-base);
  outline: none;
  cursor: pointer;
  flex: 1;
}

.model-picker {
  display: flex;
  gap: 6px;
  align-items: center;
  flex: 1;
  min-width: 0;
}

.field-hint {
  font-size: 10px;
  color: var(--text-muted);
  padding-left: 110px;
  line-height: 1.4;
}
.field-hint code {
  font-family: monospace;
  background: var(--bg-overlay);
  padding: 0 3px;
  border-radius: 2px;
}

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

.running-info {
  display: flex;
  gap: 16px;
  padding: 8px 0 0;
  font-size: 11px;
  color: var(--text-muted);
  border-top: 1px solid var(--border-subtle);
  margin-top: 4px;
}
.running-info code {
  font-family: monospace;
  color: var(--text-secondary);
}
</style>
