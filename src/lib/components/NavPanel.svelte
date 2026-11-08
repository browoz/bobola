<script lang="ts">
  import type { Snippet } from 'svelte';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import MoreHorizontal from '@lucide/svelte/icons/more-horizontal';
  import PanelsTopLeft from '@lucide/svelte/icons/panels-top-left';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Search from '@lucide/svelte/icons/search';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import Timer from '@lucide/svelte/icons/timer';
  import X from '@lucide/svelte/icons/x';
  import Zap from '@lucide/svelte/icons/zap';
  import Music2 from '@lucide/svelte/icons/music-2';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Concentration from './Concentration.svelte';
  import ServiceIcon from './ServiceIcon.svelte';
  import type { createNav } from '../nav.svelte';
  import type { Runtime, Service, Workspace } from '../types';

  let {
    nav,
    expanded,
    float,
    workspaces,
    allServices,
    services,
    selected,
    runtime,
    workspace,
    split,
    dock,
    onsearch,
    onadd,
    onsettings,
    onsplit,
    onworkspace,
    onnewworkspace,
    onactivate,
    onreload,
    onmanage,
    onclose,
    onmusic,
    ondraw,
  }: {
    nav: ReturnType<typeof createNav>;
    expanded: boolean;
    /** True when the expanded panel overlays website slots and needs a native exclusion. */
    float: boolean;
    workspaces: Workspace[];
    allServices: Service[];
    services: Service[];
    selected: string[];
    runtime: Record<string, Runtime>;
    workspace: string;
    split: boolean;
    dock?: Snippet;
    onsearch: () => void;
    onadd: () => void;
    onsettings: () => void;
    onsplit: () => void;
    onworkspace: (id: string) => void;
    onnewworkspace: () => void;
    onactivate: (id: string) => void;
    onreload: (service: Service) => void;
    onmanage: (service: Service) => void;
    onclose: (service: Service) => void;
    onmusic: () => void;
    ondraw: () => void;
  } = $props();

  let panel = $state<HTMLElement>();
  let concentrationOpen = $state(false);
  // First code point, so a name starting with an emoji or surrogate pair is not split.
  const initial = (name: string) => ([...name.trim()][0] ?? '?').toUpperCase();
  // Keyboard focus reveals the panel; pointer focus does not, so clicks cannot pin it by accident.
  function focusin(event: FocusEvent) {
    if ((event.target as HTMLElement).matches(':focus-visible')) nav.reveal();
  }
  // Focus moving into a native website arrives as a window blur, handled by the app.
  function focusout(event: FocusEvent) {
    const next = event.relatedTarget as Node | null;
    if (next && !panel?.contains(next)) nav.dismiss();
  }
</script>

<aside
  class="nav"
  class:expanded
  bind:this={panel}
  aria-label="Workspace navigation"
  data-exclusion={float ? '' : undefined}
  onpointerenter={() => nav.pointerEnter()}
  onpointerleave={() => nav.pointerLeave()}
  onfocusin={focusin}
  onfocusout={focusout}
>
  <div class="nav-scroll">
    <button
      type="button"
      class="nav-item nav-brand"
      title="Bobola settings"
      aria-label="Bobola settings"
      onclick={onsettings}
    >
      <span class="nav-icon"><img src="/app-icon.svg" alt="" width="28" height="28" /></span>
      {#if expanded}<span class="nav-text nav-wordmark">Bobola</span>{/if}
    </button>
    <button
      type="button"
      class="nav-item"
      title="Quick switch (Ctrl+K)"
      aria-label="Quick switch"
      aria-keyshortcuts="Control+K"
      onclick={onsearch}
    >
      <span class="nav-icon" aria-hidden="true"><Search size={17} /></span>
      {#if expanded}<span class="nav-text">Quick switch</span><kbd>Ctrl K</kbd>{/if}
    </button>
    {#if expanded}
      <div class="nav-label">
        Workspaces
        <button
          type="button"
          class="icon small"
          aria-label="Create workspace"
          onclick={onnewworkspace}><Plus size={14} /></button
        >
      </div>
      <button
        type="button"
        class="nav-item"
        class:active={workspace === 'all'}
        onclick={() => onworkspace('all')}
      >
        <span class="nav-icon"><LayoutGrid size={16} /></span>
        <span class="nav-text">All services</span>
        <span class="nav-count">{allServices.length}</span>
      </button>
      {#each workspaces as folder (folder.id)}
        <button
          type="button"
          class="nav-item"
          class:active={workspace === folder.id}
          onclick={() => onworkspace(folder.id)}
        >
          <span class="nav-icon"><span class="workspace-dot"></span></span>
          <span class="nav-text">{folder.name}</span>
          <span class="nav-count"
            >{allServices.filter((service) => service.workspaceId === folder.id).length}</span
          >
        </button>
      {/each}
      <div class="nav-label">Services</div>
    {:else}
      <div class="nav-sep" role="presentation"></div>
    {/if}
    <div class="nav-tools" aria-label="Bobola companions">
      <button type="button" class="nav-item companion-tool" title="Open Music" aria-label="Open Music" onclick={onmusic}>
        <span class="nav-icon"><Music2 size={17} /></span>
        {#if expanded}<span class="nav-text">Music</span><small>YuTuTui</small>{/if}
      </button>
      <button type="button" class="nav-item companion-tool" title="Open QuickDraw canvas" aria-label="Open QuickDraw canvas" onclick={ondraw}>
        <span class="nav-icon"><Pencil size={17} /></span>
        {#if expanded}<span class="nav-text">Canvas</span><small>QuickDraw</small>{/if}
      </button>
    </div>
    {#each services as service (service.id)}
      {@const isSelected = selected.includes(service.id)}
      <div class="service-item" class:selected={isSelected}>
        <button
          type="button"
          class="nav-item"
          title={service.name}
          aria-label={`${service.name}${runtime[service.id]?.loaded ? ', live' : ''}`}
          aria-current={isSelected ? 'true' : undefined}
          onclick={() => onactivate(service.id)}
        >
          <!-- Bundled vector logo with zero remote requests; falls back to initial avatar. -->
          <span class="nav-icon" aria-hidden="true"
            ><ServiceIcon
              name={service.name}
              url={service.url}
              catalogId={service.catalogId}
            /></span
          >
          {#if expanded}
            <span class="nav-text"
              >{service.name}<small>{runtime[service.id]?.loaded ? 'Live' : 'Ready to open'}</small
              ></span
            >
            {#if service.keepLive}<span class="keep" title="Keep live"><Zap size={12} /></span>{/if}
          {/if}
        </button>
        {#if expanded}
          <span class="row-actions">
            <button
              type="button"
              class="icon small"
              aria-label={`Manage ${service.name}`}
              title="Manage"
              onclick={() => onmanage(service)}><MoreHorizontal size={15} /></button
            >
            {#if isSelected}
              <button
                type="button"
                class="icon small"
                aria-label={`Reload ${service.name}`}
                title="Reload"
                onclick={() => onreload(service)}><RefreshCw size={14} /></button
              >
              <button
                type="button"
                class="icon small"
                aria-label={`Close ${service.name} view`}
                title="Close view"
                onclick={() => onclose(service)}><X size={14} /></button
              >
            {/if}
          </span>
        {/if}
      </div>
    {:else}
      {#if expanded}<p class="nav-empty">Your services will appear here.</p>{/if}
    {/each}
  </div>
  <div class="nav-foot">
    <button
      type="button"
      class="nav-item"
      class:active={split}
      aria-pressed={split}
      title="Split view"
      aria-label="Split view"
      onclick={onsplit}
    >
      <span class="nav-icon" aria-hidden="true"><PanelsTopLeft size={17} /></span>
      {#if expanded}<span class="nav-text">Split view</span>{/if}
    </button>
    <button
      type="button"
      class="nav-item"
      aria-expanded={concentrationOpen}
      title="Open concentration"
      aria-label="Open concentration"
      onclick={() => (concentrationOpen = !concentrationOpen)}
    >
      <span class="nav-icon" aria-hidden="true"><Timer size={17} /></span>
      {#if expanded}<span class="nav-text">Concentration</span>{/if}
    </button>
    <div class="nav-concentration-anchor">
      {#if concentrationOpen}
        <Concentration onclose={() => (concentrationOpen = false)} />
      {/if}
    </div>
    <button
      type="button"
      class="nav-item"
      title="Add service"
      aria-label="Add service"
      onclick={onadd}
    >
      <span class="nav-icon" aria-hidden="true"><Plus size={17} /></span>
      {#if expanded}<span class="nav-text">Add service</span>{/if}
    </button>
    <button
      type="button"
      class="nav-item"
      title="Settings"
      aria-label="Settings"
      onclick={onsettings}
    >
      <span class="nav-icon" aria-hidden="true"><Settings2 size={17} /></span>
      {#if expanded}<span class="nav-text">Settings</span>{/if}
    </button>
    {#if dock}<div class="dock">{@render dock()}</div>{/if}
  </div>
</aside>
