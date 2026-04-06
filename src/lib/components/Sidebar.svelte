<script lang="ts">
  import type { PageId } from "../types";

  interface Props {
    activePage: PageId;
    onNavigate: (page: PageId) => void;
  }

  let { activePage, onNavigate }: Props = $props();

  const navItems: { id: PageId; label: string }[] = [
    { id: "instances", label: "模型实例" },
    { id: "models",    label: "模型库" },
    { id: "proxy",     label: "代理"   },
    { id: "settings",  label: "设置"   },
  ];
</script>

<nav class="sidebar flex h-full w-44 shrink-0 flex-col">
  <!-- Logo -->
  <div class="logo-area">
    <div class="logo-badge">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 2L2 7l10 5 10-5-10-5z"/>
        <path d="M2 17l10 5 10-5"/>
        <path d="M2 12l10 5 10-5"/>
      </svg>
    </div>
    <div class="logo-text">
      <span class="logo-name">LlamaUI</span>
      <span class="logo-tag">Local AI</span>
    </div>
  </div>

  <!-- Divider -->
  <div class="divider"></div>

  <!-- Nav -->
  <div class="nav-list">
    {#each navItems as item}
      <button
        onclick={() => onNavigate(item.id)}
        class="nav-item"
        class:active={activePage === item.id}
      >
        <span class="nav-icon">
          {#if item.id === "instances"}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"/>
              <path d="m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"/>
              <path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"/>
              <path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"/>
            </svg>
          {:else if item.id === "models"}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <ellipse cx="12" cy="5" rx="9" ry="3"/>
              <path d="M3 5V19A9 3 0 0 0 21 19V5"/>
              <path d="M3 12A9 3 0 0 0 21 12"/>
            </svg>
          {:else if item.id === "proxy"}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3"/>
              <path d="M12 2v3M12 19v3M4.22 4.22l2.12 2.12M17.66 17.66l2.12 2.12M2 12h3M19 12h3M4.22 19.78l2.12-2.12M17.66 6.34l2.12-2.12"/>
            </svg>
          {:else if item.id === "settings"}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
              <circle cx="12" cy="12" r="3"/>
            </svg>
          {/if}
        </span>
        <span class="nav-label">{item.label}</span>
      </button>
    {/each}
  </div>

  <!-- Footer -->
  <div class="footer">
    <div class="divider"></div>
    <span class="version">v0.1.0</span>
  </div>
</nav>

<style>
  .sidebar {
    background: var(--bg-surface);
    border-right: 1px solid var(--border-subtle);
    padding: 14px 0 12px;
  }

  /* ─ Logo ─ */
  .logo-area {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px 14px;
  }
  .logo-badge {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: linear-gradient(135deg, #3b82f6 0%, #6366f1 100%);
    color: #fff;
    box-shadow: 0 2px 8px rgba(99, 102, 241, 0.35);
  }
  .logo-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .logo-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-base);
    line-height: 1.2;
  }
  .logo-tag {
    font-size: 10px;
    color: var(--text-muted);
    line-height: 1.2;
  }

  /* ─ Divider ─ */
  .divider {
    height: 1px;
    margin: 0 12px;
    background: var(--border-subtle);
  }

  /* ─ Nav ─ */
  .nav-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 8px 0;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 7px;
    border: 1px solid transparent;
    text-align: left;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    background: none;
    cursor: pointer;
    transition: background 0.13s, color 0.13s, border-color 0.13s;
  }
  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-base);
    border-color: var(--border-subtle);
  }
  .nav-item.active {
    background: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    border-color: rgba(59, 130, 246, 0.2);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: -1px;
    top: 22%;
    height: 56%;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: #3b82f6;
  }
  .nav-item.active:hover {
    background: rgba(59, 130, 246, 0.14);
  }

  /* ─ Icon ─ */
  .nav-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    opacity: 0.7;
    transition: opacity 0.13s;
  }
  .nav-item:hover .nav-icon,
  .nav-item.active .nav-icon {
    opacity: 1;
  }

  /* ─ Label ─ */
  .nav-label {
    line-height: 1;
  }

  /* ─ Footer ─ */
  .footer {
    padding-top: 10px;
  }
  .footer .divider {
    margin-bottom: 10px;
  }
  .version {
    display: block;
    padding: 0 14px;
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
