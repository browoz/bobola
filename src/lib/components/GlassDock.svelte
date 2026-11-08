<script lang="ts">
  import type { Service, Runtime } from '../types';
  import { tick } from 'svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import Search from '@lucide/svelte/icons/search';
  import Plus from '@lucide/svelte/icons/plus';
  import MoreHorizontal from '@lucide/svelte/icons/more-horizontal';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import X from '@lucide/svelte/icons/x';
  import Settings2 from '@lucide/svelte/icons/settings-2';

  let {
    services,
    allServices,
    selected,
    runtime,
    workspace,
    position = 'left',
    glassIntensity = 'medium',
    onsearch,
    onadd,
    onactivate,
    onreload,
    onmanage,
    onclose,
    onsettings,
    onlayout,
  }: {
    services: Service[];
    allServices: Service[];
    selected: string[];
    runtime: Record<string, Runtime>;
    workspace: string;
    position?: 'top-center' | 'top-left' | 'left';
    glassIntensity?: 'low' | 'medium' | 'high';
    onsearch: () => void;
    onadd: () => void;
    onactivate: (id: string) => void;
    onreload: (service: Service) => void;
    onmanage: (service: Service) => void;
    onclose: (service: Service) => void;
    onsettings: () => void;
    onlayout: () => unknown;
  } = $props();

  const maxVisible = 8;
  const visibleServices = $derived(services.slice(0, maxVisible));
  const overflowServices = $derived(services.slice(maxVisible));
  let overflowOpen = $state(false);
  let contextService = $state<Service | null>(null);
  $effect(() => {
    overflowOpen;
    contextService;
    services.length;
    void tick().then(onlayout);
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape') {
      overflowOpen = false;
      contextService = null;
    }
  }}
/>
<div
  class="glass-dock {position} intensity-{glassIntensity}"
  role="toolbar"
  aria-label="Floating Glass Dock"
  data-exclusion
>
  <div class="dock-stack">
    <button
      type="button"
      class="dock-item dock-brand"
      title="Bobola settings"
      aria-label="Bobola settings"
      onclick={onsettings}><img src="/app-icon.svg" width="28" height="28" alt="" /></button
    >
    <button
      type="button"
      class="dock-item search-trigger"
      title="Quick switch (Ctrl+K)"
      aria-label="Quick switch"
      onclick={onsearch}
    >
      <Search size={16} />
    </button>
    <div class="dock-sep"></div>

    {#each visibleServices as service (service.id)}
      {@const isSelected = selected.includes(service.id)}
      <div class="dock-item-wrapper" class:selected={isSelected}>
        <button
          type="button"
          class="dock-item"
          title={service.name}
          aria-label={service.name}
          aria-current={isSelected ? 'true' : undefined}
          onclick={() => onactivate(service.id)}
          oncontextmenu={(event) => {
            event.preventDefault();
            contextService = service;
            overflowOpen = false;
          }}
        >
          <span class="dock-icon"
            ><ServiceIcon
              name={service.name}
              url={service.url}
              catalogId={service.catalogId}
            /></span
          >
          {#if isSelected}<span class="active-dot" aria-hidden="true"></span>{/if}
        </button>
        {#if isSelected}
          <div class="dock-popover-actions">
            <button type="button" title="Reload" onclick={() => onreload(service)}
              ><RefreshCw size={12} /></button
            >
            <button type="button" title="Manage" onclick={() => onmanage(service)}
              ><MoreHorizontal size={13} /></button
            >
            <button type="button" title="Close" onclick={() => onclose(service)}
              ><X size={12} /></button
            >
          </div>
        {/if}
      </div>
    {:else}
      <span class="dock-empty" title="No services">No services</span>
    {/each}

    {#if overflowServices.length > 0}
      <div class="dock-sep"></div>
      <div class="overflow-wrapper">
        <button
          type="button"
          class="dock-item overflow-btn"
          aria-label="More services"
          onclick={() => (overflowOpen = !overflowOpen)}
        >
          <span class="overflow-count">+{overflowServices.length}</span>
        </button>
        {#if overflowOpen}
          <div class="overflow-dropdown" data-exclusion>
            {#each overflowServices as service (service.id)}
              {@const isSelected = selected.includes(service.id)}
              <button
                type="button"
                class="overflow-row"
                class:selected={isSelected}
                onclick={() => {
                  onactivate(service.id);
                  overflowOpen = false;
                }}
              >
                <ServiceIcon name={service.name} url={service.url} catalogId={service.catalogId} />
                <span>{service.name}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="dock-sep"></div>
    <button
      type="button"
      class="dock-item add-trigger"
      title="Add service"
      aria-label="Add service"
      onclick={onadd}
    >
      <Plus size={16} />
    </button>
    <button
      type="button"
      class="dock-item"
      aria-label="Workspace settings"
      title="Settings"
      onclick={onsettings}
    >
      <Settings2 size={16} />
    </button>
  </div>
  {#if contextService}
    {@const service = contextService}
    <div class="dock-context" data-exclusion role="group" aria-label={`${service.name} actions`}>
      <strong>{service.name}</strong>
      <button
        onclick={() => {
          onreload(service);
          contextService = null;
        }}><RefreshCw size={14} />Reload</button
      >
      <button
        onclick={() => {
          onmanage(service);
          contextService = null;
        }}><MoreHorizontal size={14} />Manage</button
      >
      <button
        onclick={() => {
          onclose(service);
          contextService = null;
        }}><X size={14} />Close view</button
      >
    </div>
  {/if}
</div>

<style>
  .glass-dock {
    position: fixed;
    z-index: 1000;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px;
    background: rgba(255, 255, 255, 0.62);
    border: 1px solid rgba(255, 255, 255, 0.4);
    box-shadow:
      0 6px 18px rgba(0, 0, 0, 0.07),
      inset 0 1px 0 rgba(255, 255, 255, 0.65);
    border-radius: 18px;
  }
  :global([data-theme='dark']) .glass-dock {
    background: rgba(18, 18, 24, 0.75);
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow:
      0 6px 18px rgba(0, 0, 0, 0.2),
      inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }
  .dock-stack {
    display: flex;
    gap: 7px;
    align-items: center;
  }
  .glass-dock.top-center {
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
  }
  .glass-dock.top-left {
    top: 16px;
    left: 20px;
  }
  .glass-dock.left {
    top: 50%;
    left: 2px;
    transform: translateY(-50%);
    flex-direction: column;
    max-height: calc(100dvh - 32px);
    border-radius: 18px;
    padding: 4px;
  }
  .left .dock-stack {
    flex-direction: column;
    overflow-y: auto;
    max-height: calc(100dvh - 48px);
    padding: 4px;
    scrollbar-width: none;
  }

  .dock-item {
    position: relative;
    width: 36px;
    height: 36px;
    padding: 0;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: rgba(255, 255, 255, 0.5);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.06),
      inset 0 1px 0 rgba(255, 255, 255, 0.6);
    border-radius: 11px;
    cursor: pointer;
    color: inherit;
    transition: transform 100ms ease;
  }
  :global([data-theme='dark']) .dock-item {
    background: rgba(40, 40, 50, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.2),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  .dock-item:hover {
    transform: scale(1.08);
    background: rgba(255, 255, 255, 0.85);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  }
  :global([data-theme='dark']) .dock-item:hover {
    background: rgba(70, 70, 90, 0.85);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }
  .dock-item-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .dock-item-wrapper.selected .dock-item {
    background: rgba(0, 0, 0, 0.08);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }
  :global([data-theme='dark']) .dock-item-wrapper.selected .dock-item {
    background: rgba(255, 255, 255, 0.15);
  }
  .active-dot {
    position: absolute;
    bottom: 2px;
    width: 5px;
    height: 5px;
    background: var(--accent);
    border-radius: 50%;
  }
  .dock-sep {
    width: 1px;
    height: 20px;
    background: rgba(0, 0, 0, 0.1);
    margin: 0 2px;
  }
  :global([data-theme='dark']) .dock-sep {
    background: rgba(255, 255, 255, 0.15);
  }
  .glass-dock.left .dock-sep {
    width: 20px;
    height: 1px;
    margin: 2px 0;
  }
  .dock-empty {
    font-size: 11px;
    opacity: 0.6;
    padding: 0 6px;
  }
  .overflow-wrapper {
    position: relative;
  }
  .overflow-btn {
    font-size: 12px;
    font-weight: 600;
  }
  .overflow-dropdown {
    position: fixed;
    top: 50%;
    left: 60px;
    transform: translateY(-50%);
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid rgba(255, 255, 255, 0.4);
    border-radius: 12px;
    box-shadow: 0 3px 10px #00000018;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 140px;
    max-width: min(260px, calc(100vw - 70px));
    max-height: calc(100dvh - 32px);
    overflow: auto;
    z-index: 1001;
  }
  :global([data-theme='dark']) .overflow-dropdown {
    background: rgba(35, 35, 42, 0.9);
    border-color: rgba(255, 255, 255, 0.1);
  }
  .overflow-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: none;
    background: transparent;
    border-radius: 8px;
    cursor: pointer;
    color: inherit;
    font-size: 13px;
    text-align: left;
    width: 100%;
  }
  .overflow-row:hover {
    background: rgba(0, 0, 0, 0.06);
  }
  :global([data-theme='dark']) .overflow-row:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .overflow-row.selected {
    font-weight: 600;
  }
  .dock-popover-actions {
    display: none;
  }
  .dock-context {
    position: fixed;
    left: 60px;
    top: 50%;
    transform: translateY(-50%);
    display: grid;
    gap: 4px;
    padding: 10px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: 0 3px 10px #00000018;
    min-width: 160px;
  }
  .dock-context button {
    display: flex;
    gap: 8px;
    align-items: center;
    border: 0;
    padding: 8px;
    background: transparent;
    color: var(--text);
    border-radius: 6px;
  }
  .dock-context button:hover {
    background: var(--accent-soft);
  }
  .dock-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  @media (prefers-reduced-motion: reduce) {
    .dock-item {
      transition: none;
    }
    .dock-item:hover {
      transform: none;
    }
  }
  :global([data-reduce-motion='true']) .dock-item {
    transition: none;
    transform: none;
  }
  .glass-dock,
  :global([data-theme='dark']) .glass-dock {
    background: var(--chrome-panel);
    color: var(--chrome-text);
    border-color: var(--chrome-line);
    box-shadow: none;
  }
  .dock-item,
  :global([data-theme='dark']) .dock-item {
    background: transparent;
    border-color: transparent;
    box-shadow: none;
  }
  .dock-item:hover,
  :global([data-theme='dark']) .dock-item:hover {
    background: var(--chrome-hover);
    box-shadow: none;
    transform: none;
  }
  .dock-item:active {
    transform: scale(0.96);
  }
  .dock-item-wrapper.selected .dock-item,
  :global([data-theme='dark']) .dock-item-wrapper.selected .dock-item {
    background: var(--chrome-hover);
    box-shadow: none;
    border-color: var(--chrome-line);
  }
  .dock-sep,
  :global([data-theme='dark']) .dock-sep {
    background: var(--chrome-line);
  }
  .overflow-dropdown,
  :global([data-theme='dark']) .overflow-dropdown,
  .dock-context {
    background: var(--chrome-panel);
    color: var(--chrome-text);
    border-color: var(--chrome-line);
  }
  .dock-context button {
    color: inherit;
  }
</style>
