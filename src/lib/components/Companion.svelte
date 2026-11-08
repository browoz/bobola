<script lang="ts">
  import { untrack } from 'svelte';
  import BobolaSprite from './BobolaSprite.svelte';
  import { REST, TICK_MS, frameOf, isRunning, reduce } from '../companion-fsm';
  import type { Fsm, FsmEvent, VerifiedCue } from '../companion-fsm';
  import type { MotionChoice } from '../interface-types';

  let {
    size,
    quiet,
    motion,
    reduced,
    paused,
    cue = null,
    ontoggle,
    onquiet,
    onmotion,
    onhide,
    onmenuchange,
  }: {
    size: number;
    quiet: boolean;
    /** The stored choice, shown in the menu. */
    motion: MotionChoice;
    /** Effective reduced motion (stored choice or system). Shows a static pose. */
    reduced: boolean;
    /** Document hidden or native window minimized. Stops the timer and resets to resting. */
    paused: boolean;
    /** Verified notification input. Nothing produces one yet. */
    cue?: VerifiedCue | null;
    ontoggle: () => void;
    onquiet: (value: boolean) => void;
    onmotion: (value: MotionChoice) => void;
    onhide: () => void;
    onmenuchange: (open: boolean) => void;
  } = $props();

  let menuOpen = $state(false);
  let menu = $state<HTMLElement>();
  let trigger = $state<HTMLButtonElement>();

  let fsm = $state.raw<Fsm>(REST);
  let pointerIn = false;
  let focusIn = false;
  let lastCue = -1;
  const animated = $derived(!reduced && !paused);
  const running = $derived(animated && isRunning(fsm));
  // Reduced motion and pause show the static sleeping pose. The reducer ignores input while paused.
  const frame = $derived(animated ? frameOf(fsm) : 'sleep');

  const send = (event: FsmEvent) => (fsm = reduce(fsm, event));
  function syncHeld() {
    const held = pointerIn || focusIn || menuOpen;
    if (held !== untrack(() => fsm.hovered) || (held && untrack(() => fsm.mode === 'sleep')))
      send({ type: held ? 'hover' : 'leave' });
  }

  // Pause and reduced motion reset to resting. Resume re-reads real hover so a pointer that is
  // still over Bobola wakes it once.
  $effect(() => {
    const live = animated;
    untrack(() => {
      send({ type: live ? 'resume' : 'pause' });
      if (live) syncHeld();
    });
  });
  // The only timer. It exists while a sequence is running and nowhere else.
  $effect(() => {
    if (!running) return;
    const timer = setInterval(() => send({ type: 'tick' }), TICK_MS);
    return () => clearInterval(timer);
  });
  $effect(() => {
    const next = cue;
    untrack(() => {
      if (!next || next.id === lastCue) return;
      lastCue = next.id;
      if (animated && !quiet) send({ type: 'cue' });
    });
  });
  $effect(() => {
    menuOpen;
    untrack(syncHeld);
  });

  function setMenu(open: boolean) {
    if (menuOpen === open) return;
    menuOpen = open;
    onmenuchange(open);
  }
  function openMenu(event: Event) {
    event.preventDefault();
    setMenu(true);
    queueMicrotask(() => menu?.querySelector<HTMLElement>('[role^="menuitem"]')?.focus());
  }
  function closeMenu(restore = false) {
    setMenu(false);
    if (restore) trigger?.focus();
  }
  function items() {
    return [...(menu?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
  }
  function menuKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.stopPropagation();
      closeMenu(true);
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const list = items();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      const index = list.indexOf(document.activeElement as HTMLElement);
      list[(index + step + list.length) % list.length]?.focus();
    } else if (event.key === 'Tab') closeMenu();
  }
  function outside(event: PointerEvent) {
    if (!menuOpen) return;
    const target = event.target as Node;
    if (!menu?.contains(target) && !(event.button === 2 && trigger?.contains(target)))
      closeMenu();
  }
  function choose(action: () => void) {
    action();
    closeMenu(true);
  }
</script>

<svelte:window onpointerdown={outside} />

<div class="cat-wrap" style:--cat-size="{size}px">
  <button
    bind:this={trigger}
    class="cat"
    type="button"
    title="Bobola: open or close the sidebar. Right-click for options."
    aria-label="Bobola, toggle sidebar"
    aria-haspopup="menu"
    aria-expanded={menuOpen}
    onclick={() => {
      ontoggle();
      send({ type: 'click' });
    }}
    oncontextmenu={openMenu}
    onpointerenter={() => {
      pointerIn = true;
      syncHeld();
    }}
    onpointerleave={() => {
      pointerIn = false;
      syncHeld();
    }}
    onfocus={(event) => {
      focusIn = (event.currentTarget as HTMLElement).matches(':focus-visible');
      syncHeld();
    }}
    onblur={() => {
      focusIn = false;
      syncHeld();
    }}
  >
    <BobolaSprite {size} {frame} />
  </button>
  {#if menuOpen}
    <div
      class="cat-menu"
      role="menu"
      aria-label="Bobola options"
      tabindex="-1"
      bind:this={menu}
      data-exclusion
      onkeydown={menuKey}
    >
      <button
        role="menuitemcheckbox"
        aria-checked={quiet}
        onclick={() => choose(() => onquiet(!quiet))}>Quiet mode</button
      >
      <div class="menu-label" role="presentation">Animation</div>
      <button
        role="menuitemradio"
        aria-checked={motion === 'system'}
        onclick={() => choose(() => onmotion('system'))}>Follow system</button
      >
      <button
        role="menuitemradio"
        aria-checked={motion === 'reduced'}
        onclick={() => choose(() => onmotion('reduced'))}>Reduced</button
      >
      <hr />
      <button role="menuitem" onclick={() => choose(onhide)}>Hide Bobola</button>
    </div>
  {/if}
</div>