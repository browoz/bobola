<script lang="ts">
  import type { Service, Runtime } from '../types';
  import ServiceIcon from './ServiceIcon.svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import { onDestroy, tick } from 'svelte';

  let {
    services,
    selected,
    runtime,
    onsearch,
    onadd,
    onactivate,
    onsettings,
    onlayout,
  }: {
    services: Service[];
    selected: string[];
    runtime: Record<string, Runtime>;
    onsearch: () => void;
    onadd: () => void;
    onactivate: (id: string) => void;
    onsettings: () => void;
    onlayout: () => unknown;
  } = $props();

  let expanded = $state(false);
  const maxVisible = 8;
  const visibleServices = $derived(services.slice(0, maxVisible));
  const radius = $derived(Math.min(200, Math.max(120, visibleServices.length * 24)));
  let leaveTimer: ReturnType<typeof setTimeout> | undefined;
  let launcher: HTMLDivElement;
  let anchor: HTMLButtonElement;
  function reveal() {
    clearTimeout(leaveTimer);
    expanded = true;
  }
  function leave() {
    clearTimeout(leaveTimer);
    leaveTimer = setTimeout(() => {
      if (!launcher?.contains(document.activeElement)) expanded = false;
    }, 250);
  }
  function dismiss() {
    clearTimeout(leaveTimer);
    expanded = false;
  }
  $effect(() => {
    expanded;
    services.length;
    void tick().then(onlayout);
  });
  onDestroy(() => clearTimeout(leaveTimer));
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && expanded) {
      dismiss();
      anchor?.focus();
    }
  }}
/>
<div
  bind:this={launcher}
  class="orb-launcher"
  class:expanded
  style={`--radius: ${radius}px`}
  role="group"
  aria-label="Minimal Orb Launcher"
  onpointerenter={(event) => {
    if (event.pointerType !== 'touch') reveal();
  }}
  onpointerleave={leave}
  onfocusin={(event) => {
    if ((event.target as HTMLElement).matches(':focus-visible') && event.target !== anchor)
      reveal();
  }}
  onfocusout={(event) => {
    if (!launcher.contains(event.relatedTarget as Node | null)) {
      dismiss();
    }
  }}
  ontransitionend={() => onlayout()}
>
  <button
    bind:this={anchor}
    type="button"
    class="orb-anchor"
    title="Service Orb Launcher"
    aria-label="Service Orb Launcher"
    aria-expanded={expanded}
    aria-controls="orb-services"
    data-exclusion
    onclick={() => (expanded = !expanded)}
    onkeydown={(event) => {
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight' || event.key === 'ArrowDown') {
        event.preventDefault();
        reveal();
        void tick().then(() => launcher.querySelector<HTMLButtonElement>('.radial-item')?.focus());
      }
    }}
  >
    <img class="orb-core" src="/app-icon.svg" width="24" height="24" alt="Bobola" />
  </button>

  <div class="radial-menu" id="orb-services" inert={!expanded} aria-hidden={!expanded}>
    <button
      type="button"
      class="radial-item orb-search"
      title="Quick switch (Ctrl+K)"
      aria-label="Quick switch"
      data-exclusion={expanded ? '' : undefined}
      onclick={onsearch}
    >
      <Search size={16} />
      <span class="radial-tooltip">Search</span>
    </button>

    {#each visibleServices as service, index (service.id)}
      {@const isSelected = selected.includes(service.id)}
      {@const angle =
        -Math.PI / 2 + ((index + 0.5) / Math.max(1, visibleServices.length)) * Math.PI}
      <button
        type="button"
        class="radial-item"
        class:selected={isSelected}
        style={`--x: ${-Math.cos(angle) * radius}px; --y: ${Math.sin(angle) * radius}px; --index: ${index}`}
        title={service.name}
        aria-label={service.name}
        aria-current={isSelected ? 'true' : undefined}
        data-exclusion={expanded ? '' : undefined}
        onclick={() => {
          onactivate(service.id);
          expanded = false;
        }}
      >
        <span class="radial-icon"
          ><ServiceIcon name={service.name} url={service.url} catalogId={service.catalogId} /></span
        >
        {#if isSelected}<span class="active-dot" aria-hidden="true"></span>{/if}
        <span class="radial-tooltip" data-exclusion={expanded ? '' : undefined}>{service.name}</span
        >
      </button>
    {/each}

    <button
      type="button"
      class="radial-item orb-add"
      title="Add service"
      aria-label="Add service"
      data-exclusion={expanded ? '' : undefined}
      onclick={onadd}
    >
      <Plus size={16} />
      <span class="radial-tooltip">Add service</span>
    </button>
    <button
      type="button"
      class="radial-item orb-settings"
      title="Workspace settings"
      aria-label="Workspace settings"
      data-exclusion={expanded ? '' : undefined}
      onclick={onsettings}
    >
      <Settings2 size={16} />
    </button>
    {#if services.length > maxVisible}
      <button
        type="button"
        class="radial-item orb-more"
        title="More services"
        aria-label="More services"
        data-exclusion={expanded ? '' : undefined}
        onclick={onsearch}>+{services.length - maxVisible}</button
      >
    {/if}
  </div>
</div>

<style>
  .orb-launcher {
    position: fixed;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    z-index: 1000;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  .orb-launcher.expanded {
    width: calc(var(--radius) + 66px);
    height: calc(var(--radius) * 2 + 64px);
  }
  .orb-anchor {
    position: absolute;
    right: 2px;
    top: calc(50% - 18px);
    width: 36px;
    height: 36px;
    padding: 0;
    pointer-events: auto;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.4);
    background: rgba(255, 255, 255, 0.45);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    cursor: pointer;
    box-shadow:
      0 4px 16px rgba(0, 0, 0, 0.1),
      0 1px 4px rgba(0, 0, 0, 0.06);
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      transform 0.2s ease,
      background 0.2s ease,
      box-shadow 0.2s ease;
  }
  :global([data-theme='dark']) .orb-anchor {
    background: rgba(35, 35, 42, 0.5);
    border-color: rgba(255, 255, 255, 0.12);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }
  .orb-anchor:hover {
    transform: scale(1.1);
    background: rgba(255, 255, 255, 0.75);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
  }
  :global([data-theme='dark']) .orb-anchor:hover {
    background: rgba(45, 45, 55, 0.8);
  }
  .orb-core {
    width: 8px;
    height: 8px;
    background: var(--accent);
    border-radius: 50%;
    box-shadow: 0 0 4px var(--accent);
    transition: transform 0.2s ease;
  }
  .orb-anchor:hover .orb-core {
    transform: scale(1.2);
  }

  .radial-menu {
    position: absolute;
    right: 2px;
    top: calc(50% - 18px);
    width: 36px;
    height: 36px;
    pointer-events: none;
  }
  .radial-item {
    position: absolute;
    left: 0;
    top: 0;
    width: 38px;
    height: 38px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.35);
    background: rgba(255, 255, 255, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    opacity: 0;
    visibility: hidden;
    transform: translate(0, 0) scale(0.2);
    transition:
      transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1),
      opacity 0.2s ease,
      background 0.15s ease;
    pointer-events: none;
    color: inherit;
  }
  :global([data-theme='dark']) .radial-item {
    background: rgba(35, 35, 42, 0.9);
    border-color: rgba(255, 255, 255, 0.12);
  }

  /* Unfold arc calculation: spreading items in a smooth quarter circle or vertical stack */
  .orb-launcher.expanded .radial-item {
    opacity: 1;
    visibility: visible;
    pointer-events: auto;
    transform: translate(var(--x, 0px), var(--y, 0px)) scale(1);
    transition-delay: calc(var(--index, 0) * 10ms);
  }

  /* Special positions for search and add */
  .orb-launcher.expanded .orb-search {
    opacity: 1;
    pointer-events: auto;
    transform: translate(-48px, -44px) scale(1);
  }
  .orb-launcher.expanded .orb-add {
    opacity: 1;
    pointer-events: auto;
    transform: translate(-48px, 0px) scale(1);
  }
  .orb-launcher.expanded .orb-settings {
    transform: translate(-48px, 44px) scale(1);
  }
  .orb-launcher.expanded .orb-more {
    transform: translate(-94px, 0px) scale(1);
  }

  .radial-item:hover {
    background: rgba(255, 255, 255, 0.95);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.15);
  }
  :global([data-theme='dark']) .radial-item:hover {
    background: rgba(50, 50, 60, 0.95);
  }
  .radial-item.selected {
    border-color: var(--accent);
  }
  .active-dot {
    position: absolute;
    bottom: 3px;
    right: 3px;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
  .radial-tooltip {
    position: absolute;
    right: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
    background: rgba(20, 20, 25, 0.85);
    color: white;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    white-space: nowrap;
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition: opacity 0.15s ease;
    backdrop-filter: blur(4px);
  }
  .radial-item:hover .radial-tooltip,
  .radial-item:focus-visible .radial-tooltip {
    opacity: 1;
    visibility: visible;
  }
  .orb-anchor:focus-visible,
  .radial-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  @media (max-height: 500px), (max-width: 320px) {
    .orb-launcher {
      --radius: 112px !important;
    }
    .orb-launcher.expanded .radial-item {
      transform: none;
      transition: opacity 150ms;
      position: static;
    }
    .orb-launcher.expanded .radial-menu {
      display: grid;
      grid-template-columns: repeat(2, 38px);
      gap: 6px;
      width: 82px;
      height: auto;
      right: 48px;
      top: 0;
      bottom: auto;
      max-height: calc(100dvh - 24px);
      overflow: auto;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .radial-item,
    .orb-anchor,
    .orb-core {
      transition: none !important;
    }
  }
  :global([data-reduce-motion='true']) .radial-item,
  :global([data-reduce-motion='true']) .orb-anchor,
  :global([data-reduce-motion='true']) .orb-core {
    transition: none !important;
  }
  .orb-launcher {
    color: var(--chrome-text);
  }
  .orb-anchor,
  :global([data-theme='dark']) .orb-anchor {
    background: var(--chrome-panel);
    border-color: var(--chrome-line);
    box-shadow: 0 4px 14px #0002;
    backdrop-filter: none;
  }
  .orb-anchor:hover,
  :global([data-theme='dark']) .orb-anchor:hover {
    background: var(--chrome-hover);
    transform: scale(1.05);
  }
  .orb-core {
    width: 26px;
    height: 26px;
    background: none;
    border-radius: 0;
    box-shadow: none;
  }
  .orb-anchor:hover .orb-core {
    transform: none;
  }
  .radial-item,
  :global([data-theme='dark']) .radial-item {
    background: var(--chrome-panel);
    color: var(--chrome-text);
    border-color: var(--chrome-line);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    transition:
      transform 200ms cubic-bezier(0.2, 0.8, 0.2, 1),
      opacity 140ms ease,
      background 140ms ease;
  }
  .radial-item:hover,
  :global([data-theme='dark']) .radial-item:hover {
    background: var(--chrome-hover);
  }
  .radial-item:active {
    filter: brightness(0.92);
  }
</style>
