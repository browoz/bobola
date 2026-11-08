<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Check from '@lucide/svelte/icons/check';
  import Download from '@lucide/svelte/icons/download';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import X from '@lucide/svelte/icons/x';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { bootstrap, command, focusService, native } from './lib/bridge';
  import type { Bootstrap, CatalogEntry, Preferences, Service } from './lib/types';
  import catalogData from './lib/catalog.json';
  import popularServices from './lib/popular-services.json';
  import ResourceSettings from './lib/components/ResourceSettings.svelte';
  import Companion from './lib/components/Companion.svelte';
  import EmptyState from './lib/components/EmptyState.svelte';
  import InterfaceSettings from './lib/components/InterfaceSettings.svelte';
  import ProtectionSettings from './lib/components/ProtectionSettings.svelte';
  import NavPanel from './lib/components/NavPanel.svelte';
  import ServiceIcon from './lib/components/ServiceIcon.svelte';
  import GlassDock from './lib/components/GlassDock.svelte';
  import OrbLauncher from './lib/components/OrbLauncher.svelte';
  import { subscribeWorkspaceEvents } from './lib/events';
  import { normalizeInterface, saveInterface } from './lib/interface-prefs';
  import type { InterfacePreferences, WorkspaceEvent } from './lib/interface-types';
  import { createNav } from './lib/nav.svelte';
  import { createSurface, measureSurface } from './lib/surface';

  const catalog = catalogData as CatalogEntry[];
  let data = $state<Bootstrap>({
    version: 1,
    services: [],
    workspaces: [],
    preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
    runtime: {},
    filterRevision: '',
    native,
  });
  let iface = $state<InterfacePreferences>(normalizeInterface(null)),
    ifaceDraft = $state<InterfacePreferences>(normalizeInterface(null)),
    ifaceLoaded = false,
    systemDark = $state(false),
    systemReduced = $state(false),
    loadFailed = $state(false),
    capability = $state<'unknown' | 'supported' | 'missing'>('unknown'),
    menuOpen = $state(false),
    mainEl = $state<HTMLElement>();
  const surface = createSurface();
  let ready = $state(false),
    error = $state(''),
    notice = $state(''),
    busy = $state(false);
  let workspace = $state('all'),
    selected = $state<string[]>([]),
    split = $state(false),
    search = $state(''),
    quickSearch = $state('');
  let modal = $state<'add' | 'settings' | 'search' | 'workspace' | 'manage' | null>(null);
  let settingsPage = $state<
    'home' | 'appearance' | 'protection' | 'notifications' | 'memory' | 'backup'
  >('home');
  const settingsSections = [
    {
      id: 'appearance',
      title: 'Appearance & navigation',
      description: 'Theme, rail size, launchers and companion',
      keywords:
        'theme dark light accent color service navigation sidebar rail density auto-hide pinned minimal floating glass dock orb launcher bobola companion cat size animation motion',
    },
    {
      id: 'protection',
      title: 'Ad & tracker protection',
      description: 'Account controls and protection lists',
      keywords: 'protection adblock ads blocking tracker shield security stats filter privacy',
    },
    {
      id: 'notifications',
      title: 'Notifications & quiet hours',
      description: 'Quiet mode, message privacy and closing',
      keywords: 'quiet mode hours notifications privacy do not disturb close',
    },
    {
      id: 'memory',
      title: 'Memory & performance',
      description: 'Live-view budget and idle accounts',
      keywords: 'memory resource live budget idle sleep performance',
    },
    {
      id: 'backup',
      title: 'Your data',
      description: 'Import and export configuration',
      keywords: 'backup export import data configuration',
    },
  ] as const;
  const blockedTotal = $derived(
    data.services.reduce((sum, service) => sum + (data.runtime[service.id]?.blocked ?? 0), 0)
  );
  let dialog: HTMLDialogElement;
  let pendingDialogCloseEvents = 0;
  let serviceName = $state(''),
    serviceUrl = $state(''),
    serviceWorkspace = $state('personal'),
    catalogSearch = $state(''),
    workspaceName = $state('');
  let pickedCatalogId = $state<string | null>(null),
    serviceKeepLive = $state(false),
    managedIdle = $state(15);
  let managedId = $state(''),
    confirmRemove = $state(false),
    settingsSearch = $state(''),
    draft = $state<Preferences>({ liveBudget: 5, idleMinutes: 15, theme: 'light' });
  // Companion timers stop while the document is hidden or the native host reports the window hidden.
  let docVisible = $state(true),
    windowVisible = $state(true);
  const companionPaused = $derived(!docVisible || !windowVisible);
  // Service named by the last quickSwitchRequested, used when the dialog is dismissed unused.
  let focusHint: string | null = null;
  // Operation started by act(), awaited before focus is restored so it never races a refresh.
  let acting: Promise<unknown> = Promise.resolve();
  let runtimeEvents = 0,
    refreshing = false,
    refreshAgain = false;
  const pinned = $derived(iface.navMode === 'pinned');
  // Floating panels overlay native website views only when the backend reports exclusion support.
  const canFloat = $derived(capability !== 'missing');
  const nav = createNav(
    () => iface.navMode,
    () => canFloat
  );
  const expanded = $derived(nav.open);
  const showCat = $derived(iface.companion.enabled && iface.navMode !== 'autohide');
  const navWidth = 182;
  const navSpace = $derived(
    iface.navMode === 'glass-dock' || iface.navMode === 'orb-launcher'
      ? 60
      : iface.navMode === 'autohide'
        ? expanded && !canFloat
          ? navWidth
          : 0
        : pinned || (expanded && !canFloat)
          ? navWidth
          : iface.density
  );
  const rightOrb = $derived(iface.navMode === 'orb-launcher');
  const resolvedTheme = $derived(
    iface.theme === 'system' ? (systemDark ? 'dark' : 'light') : iface.theme
  );
  const reduceMotion = $derived(iface.companion.motion === 'reduced' || systemReduced);
  const services = $derived(
    data.services.filter(
      (service) =>
        (workspace === 'all' || service.workspaceId === workspace) &&
        service.name.toLowerCase().includes(search.toLowerCase())
    )
  );
  const quickServices = $derived(
    data.services.filter((service) =>
      `${service.name} ${data.runtime[service.id]?.title ?? ''} ${data.workspaces.find((folder) => folder.id === service.workspaceId)?.name ?? ''}`
        .toLowerCase()
        .includes(quickSearch.toLowerCase())
    )
  );
  const chosen = $derived(
    selected
      .map((id) => data.services.find((service) => service.id === id))
      .filter((value): value is Service => !!value)
  );
  const managed = $derived(data.services.find((service) => service.id === managedId));
  const catalogResults = $derived(
    catalogSearch.trim()
      ? catalog.filter((entry) =>
          entry.name.toLowerCase().includes(catalogSearch.trim().toLowerCase())
        )
      : popularServices
  );
  const quickActions = [
    { name: 'Add service', run: () => replaceDialog('add') },
    { name: 'Settings', run: () => replaceDialog('settings') },
    {
      name: 'Memory & running accounts',
      run: async () => {
        await replaceDialog('settings');
        settingsPage = 'memory';
      },
    },
    { name: 'Sleep eligible accounts', run: () => sleepEligible() },
  ];
  async function replaceDialog(kind: NonNullable<typeof modal>) {
    // Keep modal capture engaged when changing content; avoid focus races with native views.
    modal = kind;
    if (kind === 'settings') {
      settingsPage = 'home';
      draft = { ...data.preferences };
      ifaceDraft = $state.snapshot(iface);
    }
    if (kind === 'add') serviceWorkspace = workspace === 'all' ? 'personal' : workspace;
    await tick();
    dialog.querySelector<HTMLInputElement>('input')?.focus();
  }
  function manageService(service: Service) {
    managedId = service.id;
    managedIdle = service.idleMinutes ?? data.preferences.idleMinutes;
    confirmRemove = false;
    if (modal) void replaceDialog('manage');
    else void open('manage');
  }
  async function sleepAccount(service: Service) {
    await act(() => command('unload_service', { id: service.id }));
  }
  async function sleepEligible() {
    await act(async () => {
      const eligible = data.services.filter((service) => {
        const runtime = data.runtime[service.id];
        return (
          runtime?.loaded &&
          !runtime.visible &&
          !runtime.dirty &&
          !runtime.media &&
          !service.keepLive
        );
      });
      const results = await Promise.allSettled(
        eligible.map((service) => command('unload_service', { id: service.id }))
      );
      const failed = results.filter((result) => result.status === 'rejected').length;
      notice = `${results.length - failed} accounts put to sleep.${failed ? ` ${failed} accounts stayed live because their state changed or a popup is open.` : ''}`;
    });
  }

  async function refresh() {
    const stamp = runtimeEvents;
    const next = await bootstrap();
    // A runtimeChanged event that landed during this fetch is newer than the fetched runtime.
    data = stamp === runtimeEvents ? next : { ...next, runtime: data.runtime };
    if (!ifaceLoaded) {
      iface = normalizeInterface(data.interfacePreferences, data.preferences.theme);
      ifaceLoaded = true;
    }
    loadFailed = false;
    ready = true;
  }
  // Applies immediately for this session, then persists. A failed save is reported, never hidden.
  async function updateInterface(next: InterfacePreferences) {
    iface = normalizeInterface(next);
    try {
      await saveInterface($state.snapshot(iface));
      return true;
    } catch (failure) {
      error = `Interface settings applied for this session only. ${describe(failure)}`;
      return false;
    }
  }
  const patch = (change: (value: InterfacePreferences) => void) => {
    const next = $state.snapshot(iface) as InterfacePreferences;
    change(next);
    return updateInterface(next);
  };
  function quickSwitch() {
    // An open dialog keeps its own focus; a second showModal() would throw.
    if (modal) return;
    if (iface.navMode === 'autohide') nav.reveal();
    void open('search');
  }
  // Collapses a burst of backend events into one trailing refresh.
  async function refreshSoon() {
    if (refreshing) {
      refreshAgain = true;
      return;
    }
    refreshing = true;
    try {
      do {
        refreshAgain = false;
        await refresh();
      } while (refreshAgain);
    } catch (failure) {
      error = `Workspace state could not be refreshed. ${describe(failure)}`;
    } finally {
      refreshing = false;
    }
  }
  function handleEvent(event: WorkspaceEvent) {
    switch (event.type) {
      case 'quickSwitchRequested':
        if (!modal) {
          focusHint = event.serviceId ?? null;
          quickSwitch();
        }
        break;
      case 'popupError':
        error = event.message ?? 'A popup could not be opened.';
        break;
      case 'windowVisibility':
        windowVisible = event.visible;
        break;
      case 'runtimeChanged':
        runtimeEvents++;
        data = { ...data, runtime: event.runtime };
        break;
      case 'popupOpened':
      case 'popupClosed':
      case 'snapshotChanged':
        void refreshSoon();
        break;
    }
  }
  async function saveSettings() {
    const legacyTheme =
      ifaceDraft.theme === 'system' ? (systemDark ? 'dark' : 'light') : ifaceDraft.theme;
    await act(async () => {
      await command('save_preferences', { preferences: { ...draft, theme: legacyTheme } });
      if (await updateInterface($state.snapshot(ifaceDraft) as InterfacePreferences)) close();
    });
  }
  function describe(value: unknown) {
    return value instanceof Error ? value.message : String(value);
  }
  function act(operation: () => Promise<unknown>) {
    error = '';
    busy = true;
    // Starts in a microtask so `acting` is set before the operation can dismiss a dialog.
    const run = Promise.resolve()
      .then(operation)
      .then(() => refresh())
      .catch((failure) => {
        error = describe(failure);
      })
      .finally(() => {
        busy = false;
      });
    acting = run;
    return run;
  }
  /** Resolves true once the native layout, exclusions and modal capture were applied. */
  async function syncLayout() {
    try {
      await tick();
      await surface.request(() => measureSurface(!!modal));
      capability = surface.capability;
      return true;
    } catch (failure) {
      error = `Native layout could not be applied. ${describe(failure)}`;
      return false;
    }
  }
  async function open(kind: NonNullable<typeof modal>) {
    error = '';
    if (kind === 'search') quickSearch = '';
    if (kind === 'add') serviceWorkspace = workspace === 'all' ? 'personal' : workspace;
    if (kind === 'settings') {
      settingsPage = 'home';
      settingsSearch = '';
    }
    draft = { ...data.preferences };
    ifaceDraft = $state.snapshot(iface) as InterfacePreferences;
    if (kind === 'manage') void refresh().catch(() => {});
    modal = kind;
    // Input capture must be confirmed first: otherwise a native view could cover the dialog or
    // keep the keyboard. On failure the dialog stays closed and the error strip explains why.
    if (!(await syncLayout())) {
      modal = null;
      return;
    }
    await tick();
    if (!modal || dialog.open) return;
    dialog.showModal();
    dialog.querySelector<HTMLInputElement>('input')?.focus();
  }
  // Website to focus once the current dialog is gone (an exact account activation).
  let focusAfter: string | null = null;
  // Every dismissal (buttons, Escape, programmatic) ends here exactly once.
  function finishDialog() {
    if (!modal) return;
    const target = focusAfter;
    focusAfter = null;
    modal = null;
    void restoreAfter(target);
  }
  function close() {
    if (dialog?.open) {
      // close events are queued: an old one must not dismiss a newly opening dialog.
      pendingDialogCloseEvents++;
      dialog.close();
    }
    finishDialog();
  }
  function closeAndFocus(id: string) {
    focusAfter = id;
    close();
  }
  async function restoreAfter(target: string | null) {
    await acting;
    if (await syncLayout()) await restoreFocus(target);
  }
  /**
   * Returns keyboard focus to a website view. An explicit id wins, then the service named by the
   * last quick-switch request, then the backend's own last focused service (null).
   * Skipped while a dialog or the Bobola menu is open, so it never steals focus from them.
   */
  async function restoreFocus(target: string | null) {
    if (!native || modal || menuOpen || !docVisible || !windowVisible) return;
    const hint = focusHint;
    focusHint = null;
    const id = target ?? (hint && chosen.some((s) => s.id === hint) ? hint : null);
    if (id !== null ? !chosen.some((s) => s.id === id) : !chosen.length) return;
    try {
      await focusService(id);
    } catch (failure) {
      error = `Keyboard focus could not return to the website. ${describe(failure)}`;
    }
  }
  async function focusAfterLayout(id: string) {
    if (await syncLayout()) await restoreFocus(id);
  }
  // Closes the sidebar and, when it was open, hands the keyboard back to the website.
  function dismissNav() {
    const wasOpen = !pinned && nav.open;
    nav.dismiss();
    if (wasOpen) void restoreFocus(null);
  }
  function activate(id: string) {
    nav.dismiss();
    const service = data.services.find((service) => service.id === id);
    if (workspace !== 'all' && service?.workspaceId !== workspace) workspace = 'all';
    if (!(split && selected.includes(id)))
      selected =
        split && selected.length
          ? [selected[0], id]
              .filter((value, index, list) => list.indexOf(value) === index)
              .slice(0, 2)
          : [id];
    // The newly chosen account always takes keyboard focus, even if it was already visible.
    if (modal === 'search') closeAndFocus(id);
    else void focusAfterLayout(id);
  }
  function selectWorkspace(id: string) {
    workspace = id;
    if (id !== 'all')
      selected = selected.filter((serviceId) =>
        data.services.some((service) => service.id === serviceId && service.workspaceId === id)
      );
    search = '';
  }
  function pick(entry: CatalogEntry) {
    pickedCatalogId = entry.id;
    serviceKeepLive = popularServices.find((service) => service.id === entry.id)?.keepLive ?? false;
    serviceName = entry.name;
    serviceUrl = entry.needsCustomUrl ? '' : entry.url;
  }
  async function addService(event: SubmitEvent) {
    event.preventDefault();
    await act(async () => {
      const service = await command<Service>('add_service', {
        input: {
          name: serviceName,
          url: serviceUrl,
          workspaceId: serviceWorkspace,
          keepLive: serviceKeepLive,
          catalogId: pickedCatalogId,
        },
      });
      workspace = 'all';
      selected = [service.id];
      serviceName = '';
      serviceUrl = '';
      pickedCatalogId = null;
      serviceKeepLive = false;
      closeAndFocus(service.id);
    });
  }
  async function toggleService(service: Service, property: 'protection' | 'keepLive') {
    await act(() =>
      command('update_service', { service: { ...service, [property]: !service[property] } })
    );
    if (property === 'protection')
      notice = 'Protection updated. Reload the service to reapply page styling.';
  }
  async function removeService(service: Service) {
    await act(async () => {
      await command('delete_service', { id: service.id });
      selected = selected.filter((id) => id !== service.id);
      confirmRemove = false;
      close();
    });
  }
  async function exportBackup() {
    await act(async () => {
      const text = await command<string>('export_backup');
      const href = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
      const link = document.createElement('a');
      link.href = href;
      link.download = 'bobola-backup.json';
      link.click();
      setTimeout(() => URL.revokeObjectURL(href), 1000);
      notice = 'Configuration exported. Website login sessions are stored separately.';
    });
  }
  async function importBackup(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    if (
      !confirm(
        'Replace the current configuration with this backup? Loaded services must be closed first.'
      )
    )
      return;
    await act(async () => {
      await command('import_backup', { text: await file.text() });
      selected = [];
      workspace = 'all';
      notice = 'Configuration restored.';
    });
    input.value = '';
  }
  function settingMatches(text: string) {
    return text.toLowerCase().includes(settingsSearch.toLowerCase());
  }
  function keydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      quickSwitch();
    }
    if (event.key === 'Escape' && !modal && !pinned) dismissNav();
    if (event.ctrlKey && event.key === 'Tab' && data.services.length && !modal) {
      event.preventDefault();
      const index = data.services.findIndex((service) => service.id === selected[0]);
      const next =
        data.services[
          (index + (event.shiftKey ? -1 : 1) + data.services.length) % data.services.length
        ].id;
      selected = [next];
      void focusAfterLayout(next);
    }
  }

  $effect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolvedTheme;
    root.dataset.accent = iface.accent;
    root.dataset.reduceMotion = String(reduceMotion);
    root.style.setProperty('--rail', `${iface.density}px`);
    root.style.setProperty('--nav-space', `${navSpace}px`);
    root.style.setProperty('--nav-left', rightOrb ? '0px' : `${navSpace}px`);
    root.style.setProperty('--nav-right', rightOrb ? `${navSpace}px` : '0px');
    const rgb = data.runtime[selected[0]]?.background;
    const valid =
      rgb?.length === 3 &&
      rgb.every((channel) => Number.isInteger(channel) && channel >= 0 && channel <= 255);
    root.dataset.chromeTheme = valid
      ? rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722 < 140
        ? 'dark'
        : 'light'
      : resolvedTheme;
    if (valid) root.style.setProperty('--service-background', `rgb(${rgb.join(',')})`);
    else root.style.removeProperty('--service-background');
  });
  $effect(() => {
    selected;
    split;
    modal;
    data.services;
    iface.navMode;
    iface.density;
    iface.companion.enabled;
    iface.companion.size;
    expanded;
    menuOpen;
    if (ready) void syncLayout();
  });
  $effect(() => {
    iface.navMode;
    nav.dismiss();
  });
  // Without exclusion support a menu wider than the rail would sit under the website view.
  $effect(() => {
    if (menuOpen && !canFloat) nav.reveal();
  });

  onMount(() => {
    refresh().catch((failure) => {
      error = describe(failure);
      loadFailed = true;
    });
    document.title = 'Bobola';
    const dark = matchMedia('(prefers-color-scheme: dark)');
    const reduced = matchMedia('(prefers-reduced-motion: reduce)');
    systemDark = dark.matches;
    systemReduced = reduced.matches;
    const onDark = () => (systemDark = dark.matches);
    const onReduced = () => (systemReduced = reduced.matches);
    dark.addEventListener('change', onDark);
    reduced.addEventListener('change', onReduced);
    // No polling: state refreshes after actions, on focus, and on runtimeChanged/snapshotChanged.
    const onFocus = () => {
      if (!busy) void refreshSoon();
    };
    window.addEventListener('focus', onFocus);
    docVisible = !document.hidden;
    const onVisibility = () => (docVisible = !document.hidden);
    document.addEventListener('visibilitychange', onVisibility);
    const observer = new ResizeObserver(() => {
      if (ready) void syncLayout();
    });
    observer.observe(document.body);
    if (mainEl) observer.observe(mainEl);
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    const track = (pending: Promise<() => void>) =>
      pending.then((stop) => (disposed ? stop() : unlisteners.push(stop))).catch(() => {});
    if (native) track(subscribeWorkspaceEvents(handleEvent));
    return () => {
      disposed = true;
      nav.destroy();
      dark.removeEventListener('change', onDark);
      reduced.removeEventListener('change', onReduced);
      window.removeEventListener('focus', onFocus);
      document.removeEventListener('visibilitychange', onVisibility);
      observer.disconnect();
      unlisteners.forEach((stop) => stop());
    };
  });
</script>

<svelte:window
  onkeydown={keydown}
  onblur={() => nav.dismiss()}
  onpointerdown={(event) => {
    const target = event.target as Element;
    if (pinned || target.closest('.nav, .edge')) return;
    // A press on a control or inside a dialog keeps its own focus; only empty chrome hands it back.
    if (target.closest('button, a, input, select, textarea, dialog, [role="menu"]')) nav.dismiss();
    else dismissNav();
  }}
/>

{#snippet dock()}
  <Companion
    size={iface.companion.size}
    quiet={iface.quiet.enabled}
    motion={iface.companion.motion}
    reduced={reduceMotion}
    paused={companionPaused}
    ontoggle={() => nav.toggle()}
    onquiet={(value) => patch((next) => (next.quiet.enabled = value))}
    onmotion={(value) => patch((next) => (next.companion.motion = value))}
    onhide={() => {
      notice = 'Bobola is hidden. Turn it back on in Settings.';
      void patch((next) => (next.companion.enabled = false));
    }}
    onmenuchange={(value) => (menuOpen = value)}
  />
{/snippet}

<div class="app-shell" data-nav={iface.navMode}>
  {#if iface.navMode === 'autohide'}
    <div
      class="edge"
      role="presentation"
      data-exclusion
      onpointerenter={() => nav.pointerEnter()}
      onpointerleave={() => nav.pointerLeave()}
    ></div>
  {/if}
  {#if iface.navMode === 'glass-dock'}
    <GlassDock
      position="left"
      onlayout={syncLayout}
      onsettings={() => void open('settings')}
      {services}
      allServices={data.services}
      {selected}
      runtime={data.runtime}
      {workspace}
      onsearch={() => void open('search')}
      onadd={() => void open('add')}
      onactivate={activate}
      onreload={(service) => act(() => command('reload_service', { id: service.id }))}
      onmanage={manageService}
      onclose={(service) => {
        selected = selected.filter((id) => id !== service.id);
      }}
    />
  {:else if iface.navMode === 'orb-launcher'}
    <OrbLauncher
      onlayout={syncLayout}
      onsettings={() => void open('settings')}
      {services}
      {selected}
      runtime={data.runtime}
      onsearch={() => void open('search')}
      onadd={() => void open('add')}
      onactivate={activate}
    />
  {:else}
    <NavPanel
      {nav}
      {expanded}
      float={expanded && !pinned && canFloat}
      workspaces={data.workspaces}
      allServices={data.services}
      {services}
      {selected}
      runtime={data.runtime}
      {workspace}
      {split}
      dock={showCat ? dock : undefined}
      onsearch={() => {
        nav.dismiss();
        void open('search');
      }}
      onadd={() => {
        nav.dismiss();
        void open('add');
      }}
      onsettings={() => {
        nav.dismiss();
        void open('settings');
      }}
      onsplit={() => {
        split = !split;
        if (!split) selected = selected.slice(0, 1);
      }}
      onworkspace={selectWorkspace}
      onnewworkspace={() => {
        nav.dismiss();
        void open('workspace');
      }}
      onactivate={activate}
      onreload={(service) => act(() => command('reload_service', { id: service.id }))}
      onmanage={manageService}
      onclose={(service) => {
        selected = selected.filter((id) => id !== service.id);
      }}
    />
  {/if}

  <main id="main" bind:this={mainEl}>
    {#if error}<div class="strip error" role="alert">
        <span>{error}</span><button
          class="icon small"
          aria-label="Dismiss error"
          onclick={() => {
            error = '';
          }}><X size={16} /></button
        >
      </div>{/if}
    {#if notice}<div class="strip" role="status">
        <span>{notice}</span><button
          class="icon small"
          aria-label="Dismiss message"
          onclick={() => {
            notice = '';
          }}><X size={16} /></button
        >
      </div>{/if}
    {#if !native}<div class="strip">
        <span><Globe2 size={13} /> Shell preview. Open the Windows app to use live services.</span>
      </div>{/if}
    {#if capability === 'missing' && iface.navMode === 'autohide'}<div class="strip" role="status">
        <span
          >This build cannot draw over website views, so the auto-hide edge may not receive the
          pointer. Press Ctrl+K to open navigation.</span
        >
      </div>{/if}

    {#if !ready && !loadFailed}
      <div class="state" role="status">Loading your workspace…</div>
    {:else if loadFailed}
      <div class="empty">
        <h1>Workspace could not load</h1>
        <p>{error || 'The backend did not respond.'}</p>
        <button
          type="button"
          class="button primary"
          onclick={() => refresh().catch((failure) => (error = describe(failure)))}
          ><RefreshCw size={15} />Try again</button
        >
      </div>
    {:else if !chosen.length}
      <EmptyState
        {services}
        hasAny={data.services.length > 0}
        bind:search
        onopen={activate}
        onadd={() => open('add')}
        onsuggest={(name, url) => {
          serviceName = name;
          serviceUrl = url;
          void open('add');
        }}
      />
    {:else}
      <section class="live-grid" class:two-columns={chosen.length > 1} aria-label="Open services">
        {#each chosen as service (service.id)}<article class="service-pane">
            <div class="native-slot" data-service-id={service.id}>
              {#if !native}<div class="preview-placeholder">
                  <Globe2 size={32} />
                  <h3>{service.name}</h3>
                  <p>
                    This account is saved in the shell preview.<br />Launch the Windows app to sign
                    in.
                  </p>
                </div>{/if}
            </div>
          </article>{/each}
      </section>
    {/if}
  </main>
</div>

<dialog
  bind:this={dialog}
  class:settings-dialog={modal === 'settings'}
  data-settings-page={settingsPage}
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
  onclose={() => {
    if (pendingDialogCloseEvents > 0) {
      pendingDialogCloseEvents--;
      return;
    }
    if (!dialog.open) finishDialog();
  }}
>
  <div class="modal-header">
    <div>
      <img class="modal-logo" src="/app-icon.svg" width="32" height="32" alt="Bobola" />
      <h2>
        {modal === 'add'
          ? 'Add a service'
          : modal === 'workspace'
            ? 'New workspace'
            : modal === 'search'
              ? 'Quick switch'
              : modal === 'manage'
                ? managed?.name
                : settingsPage === 'home'
                  ? 'Settings'
                  : settingsSections.find((section) => section.id === settingsPage)?.title}
      </h2>
    </div>
    <button class="icon" aria-label="Close dialog" onclick={close}><X size={20} /></button>
  </div>
  {#if error}<p class="form-error" role="alert">{error}</p>{/if}
  {#if modal === 'add'}
    <p class="modal-description">
      Give each account its own place. Your sign-in stays with that account.
    </p>
    <div class="action-row">
      <strong
        >{catalogSearch.trim()
          ? `${catalogResults.length} matching services`
          : '30 popular services'}</strong
      ><button
        class="button secondary"
        type="button"
        onclick={() => {
          pickedCatalogId = null;
          serviceName = '';
          serviceUrl = '';
          serviceKeepLive = false;
          void tick().then(() =>
            dialog.querySelector<HTMLInputElement>('input[placeholder="Gmail · Personal"]')?.focus()
          );
        }}>Custom service</button
      >
    </div>
    <label class="field"
      >Find an app<input
        type="search"
        placeholder="Search the service catalog"
        bind:value={catalogSearch}
      /></label
    >
    <div class="catalog">
      {#each catalogResults as entry}<button onclick={() => pick(entry)}
          ><ServiceIcon
            name={entry.name}
            url={entry.url}
            catalogId={entry.id}
            size={26}
            class="tiny"
          /><span>{entry.name}</span><Plus size={14} /></button
        >{:else}<p class="modal-description">
          No matching services. Use Custom service to add your website.
        </p>{/each}
    </div>
    <form onsubmit={addService}>
      <div class="form-grid">
        <label class="field"
          >Service name<input
            required
            maxlength="80"
            bind:value={serviceName}
            placeholder="Gmail · Personal"
          /></label
        ><label class="field"
          >Workspace<select bind:value={serviceWorkspace}
            >{#each data.workspaces as folder}<option value={folder.id}>{folder.name}</option
              >{/each}</select
          ></label
        >
      </div>
      <label class="field"
        >Website URL<input
          type="url"
          required
          bind:value={serviceUrl}
          placeholder="https://example.com"
        /></label
      >
      <label class="field"
        >Background activity<select bind:value={serviceKeepLive}
          ><option value={false}>Sleep when idle · lower memory use</option><option value={true}
            >Stay live after opening · receive notifications</option
          ></select
        ></label
      >
      <p class="field-note">
        <ShieldCheck size={14} />Protection is enabled. Sleeping pauses notifications and keeps your
        sign-in. Wake-up reloads the page; unsaved page state is not restored.
      </p>
      <div class="modal-footer">
        <button class="button secondary" type="button" onclick={close}>Cancel</button><button
          class="button primary"
          disabled={busy}
          type="submit">{busy ? 'Adding…' : 'Add service'}<ArrowRight size={16} /></button
        >
      </div>
    </form>
  {:else if modal === 'workspace'}
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void act(async () => {
          await command('add_workspace', { name: workspaceName });
          workspaceName = '';
          close();
        });
      }}
    >
      <label class="field"
        >Workspace name<input
          required
          maxlength="80"
          placeholder="Work, Personal, Side projects…"
          bind:value={workspaceName}
        /></label
      >
      <div class="modal-footer">
        <button class="button primary" disabled={busy}>Create workspace<Plus size={16} /></button>
      </div>
    </form>
  {:else if modal === 'search'}
    <label class="field"
      >Find a service<input
        type="search"
        bind:value={quickSearch}
        placeholder="Type a service name…"
      /></label
    >
    <div class="quick-results">
      {#each quickServices as service}<div class="quick-service-row">
          <button onclick={() => activate(service.id)}
            ><ServiceIcon
              name={service.name}
              url={service.url}
              catalogId={service.catalogId}
              size={33}
            /><span
              >{service.name}<small
                >{data.runtime[service.id]?.title || new URL(service.url).hostname}</small
              ></span
            ><ArrowRight size={16} /></button
          ><button
            class="text-button"
            aria-label={`Manage ${service.name}`}
            onclick={() => manageService(service)}>Manage</button
          >
        </div>
      {:else}<p class="modal-description">No matching services yet.</p>{/each}
      {#each quickActions.filter((action) => action.name
          .toLowerCase()
          .includes(quickSearch.toLowerCase())) as action}<button onclick={action.run}
          ><span>{action.name}</span><ArrowRight size={16} /></button
        >{/each}
    </div>
  {:else if modal === 'manage' && managed}
    {#if confirmRemove}
      <p class="modal-description">
        Remove {managed.name} from your Bobola service list? Its browser profile and sign-in data will
        remain on this PC.
      </p>
      <div class="modal-footer">
        <button class="button secondary" type="button" onclick={() => (confirmRemove = false)}
          >Keep service</button
        >
        <button
          class="button danger"
          type="button"
          disabled={busy}
          onclick={() => removeService(managed)}
          ><Trash2 size={15} />{busy ? 'Removing…' : 'Remove service'}</button
        >
      </div>
    {:else}
      <p class="modal-description">{managed.url}</p>
      <div class="setting-row">
        <div>
          <strong>Protection</strong>
          <p>Block matching requests and hide matching page elements.</p>
        </div>
        <button
          class="toggle"
          role="switch"
          aria-checked={managed.protection}
          aria-label="Service protection"
          onclick={() => managed && toggleService(managed, 'protection')}><span></span></button
        >
      </div>
      <div class="setting-row">
        <div>
          <strong>Keep live</strong>
          <p>
            Once opened, keep this account live for notifications. Otherwise it sleeps when idle;
            calls and unsaved work stay protected.
          </p>
        </div>
        <button
          class="toggle"
          role="switch"
          aria-checked={managed.keepLive}
          aria-label="Keep service live"
          onclick={() => managed && toggleService(managed, 'keepLive')}><span></span></button
        >
      </div>
      {#if !managed.keepLive}<div class="setting-row">
          <div>
            <strong>Sleep after idle</strong>
            <p>
              Minutes. The live-view budget can put eligible background accounts to sleep earlier.
            </p>
          </div>
          <input
            class="number-input"
            type="number"
            aria-label="Account idle minutes"
            min="1"
            max="240"
            bind:value={managedIdle}
          /><button
            class="text-button"
            disabled={busy ||
              !Number.isInteger(managedIdle) ||
              managedIdle < 1 ||
              managedIdle > 240}
            onclick={() =>
              act(() =>
                command('update_service', { service: { ...managed, idleMinutes: managedIdle } })
              )}>Apply</button
          >
        </div>{/if}
      <div class="action-row">
        <button
          class="button secondary"
          disabled={busy}
          onclick={() => act(() => command('reload_service', { id: managedId }))}
          ><RefreshCw size={15} />Reload service</button
        >
      </div>
      <div class="manage-note">
        <ShieldCheck size={18} />
        <p>
          {data.runtime[managed.id]?.blocked ?? 0} requests blocked in this account this session.
        </p>
      </div>
      <div class="modal-footer split-footer">
        <button class="button danger" type="button" onclick={() => (confirmRemove = true)}
          ><Trash2 size={15} />Remove service</button
        ><button
          class="button secondary"
          onclick={() =>
            act(async () => {
              selected = selected.filter((id) => id !== managedId);
              if (!(await syncLayout()))
                throw new Error('Could not hide the account before sleeping.');
              await command('unload_service', { id: managedId });
              close();
            })}>Sleep account</button
        >
      </div>
    {/if}
  {:else if modal === 'settings'}
    {#if settingsPage === 'home'}
      <label class="field"
        >Search settings<input
          type="search"
          bind:value={settingsSearch}
          placeholder="Protection, memory, appearance, backup…"
        /></label
      >
      <button class="protection-summary" onclick={() => (settingsPage = 'protection')}>
        <ShieldCheck size={24} />
        <span
          ><strong>{native ? blockedTotal.toLocaleString() : 'Ad & tracker protection'}</strong
          ><small
            >{native
              ? 'network requests blocked across current accounts this app session.'
              : 'Manage protection for your accounts'}</small
          ></span
        >
        <ChevronRight size={17} />
      </button>
      <div class="settings-sections">
        {#each settingsSections.filter( (section) => settingMatches(`${section.title} ${section.description} ${section.keywords}`) ) as section}
          <button class="settings-link" onclick={() => (settingsPage = section.id)}
            ><span><strong>{section.title}</strong><small>{section.description}</small></span
            ><ChevronRight size={16} /></button
          >
        {:else}<p class="modal-description">No matching settings.</p>{/each}
      </div>
    {:else}
      <button class="settings-back" onclick={() => (settingsPage = 'home')}
        ><ChevronLeft size={15} />All settings</button
      >
      {#if settingsPage === 'appearance' || settingsPage === 'notifications'}
        <InterfaceSettings
          bind:draft={ifaceDraft}
          show={(text) =>
            settingsPage === 'notifications'
              ? text.includes('quiet mode')
              : !text.includes('quiet mode')}
          {native}
        />
      {/if}
      {#if settingsPage === 'memory'}<ResourceSettings
          services={data.services}
          runtime={data.runtime}
          {native}
          {busy}
          onmanage={manageService}
          onsleep={sleepAccount}
        />
        <div class="setting-row">
          <div>
            <strong>Live-view budget</strong>
            <p>Calls, unsaved work, and keep-live accounts take priority.</p>
          </div>
          <input
            class="number-input"
            aria-label="Live-view budget"
            type="number"
            min="1"
            max="30"
            bind:value={draft.liveBudget}
          />
        </div>
        <div class="setting-row">
          <div>
            <strong>Idle period</strong>
            <p>Minutes before eligible background accounts unload.</p>
          </div>
          <input
            class="number-input"
            aria-label="Idle period in minutes"
            type="number"
            min="1"
            max="240"
            bind:value={draft.idleMinutes}
          />
        </div>{/if}
      {#if settingsPage === 'protection'}
        <ProtectionSettings
          services={data.services}
          runtime={data.runtime}
          {native}
          {busy}
          onprotection={(service) => toggleService(service, 'protection')}
        />
        <div class="settings-group">
          <div class="section-heading">
            <h3>Protection lists</h3>
            <ShieldCheck size={18} />
          </div>
          <p>
            Includes a small starter list. Download EasyList and EasyPrivacy for broader coverage.
          </p>
          <div class="action-row">
            <button
              class="button secondary"
              disabled={busy || !native}
              onclick={() =>
                act(async () => {
                  await command('refresh_filters');
                  notice = 'Protection lists updated.';
                })}><RefreshCw size={15} />Update lists</button
            ><button
              class="text-button"
              disabled={busy || !native}
              onclick={() =>
                act(async () => {
                  await command('rollback_filters');
                  notice = 'Previous protection lists restored.';
                })}>Restore previous lists</button
            >
          </div>
        </div>{/if}
      {#if settingsPage === 'backup'}<div class="settings-group">
          <h3>Your data</h3>
          <p>Back up app configuration. Website passwords and login sessions are not included.</p>
          <div class="action-row">
            <button class="button secondary" onclick={exportBackup}
              ><Download size={15} />Export configuration</button
            ><label class="button secondary upload-label"
              >Import<input
                type="file"
                accept="application/json,.json"
                onchange={importBackup}
                disabled={!native}
              /></label
            >
          </div>
        </div>{/if}
    {/if}
    <div class="modal-footer">
      <button class="button secondary" onclick={close}>Cancel</button><button
        class="button primary"
        disabled={busy}
        onclick={saveSettings}><Check size={16} />Save changes</button
      >
    </div>
  {/if}
</dialog>
