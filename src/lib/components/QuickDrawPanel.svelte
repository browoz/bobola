<script lang="ts">
  import { onMount } from 'svelte';
  import Eraser from '@lucide/svelte/icons/eraser';
  import Moon from '@lucide/svelte/icons/moon';
  import Pencil from '@lucide/svelte/icons/pencil';
  import Sun from '@lucide/svelte/icons/sun';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import { createQuickdraw } from '@quickdrawjs/core';
  import '@quickdrawjs/core/quickdraw.css';

  let { theme = 'light' }: { theme?: 'light' | 'dark' } = $props();
  let board = $state<HTMLElement>();
  let boardApi: ReturnType<typeof createQuickdraw> | undefined;
  let boardTheme = $state<'light' | 'dark'>('light');

  onMount(() => {
    if (!board) return;
    let saved: unknown;
    try { saved = JSON.parse(localStorage.getItem('bobola-quickdraw-board') ?? 'null'); } catch { saved = undefined; }
    boardTheme = theme ?? 'light';
    boardApi = createQuickdraw({ container: board, theme: boardTheme, grid: 'lines', watermark: true });
    if (saved) boardApi.editor.store.loadSnapshot(saved as never);
    boardApi.editor.store.listen(() => localStorage.setItem('bobola-quickdraw-board', JSON.stringify(boardApi?.editor.store.getSnapshot())));
    return () => boardApi?.destroy();
  });
  function setTheme(next: 'light' | 'dark') { boardTheme = next; boardApi?.editor.setTheme(next); }
  function undo() { boardApi?.editor.store.undo(); }
  function clear() { boardApi?.editor.store.clear(); }
</script>

<section class="draw-panel" data-theme={boardTheme} aria-label="Bobola QuickDraw canvas">
  <header class="draw-header">
    <div><span class="draw-kicker"><Pencil size={14} /> QUICKDRAW / CANVAS</span><h2>Think in public.</h2><p>A local infinite canvas for plans, diagrams, and rough ideas.</p></div>
    <div class="draw-actions">
      <button aria-label="Undo" title="Undo" onclick={undo}><Undo2 size={16} /></button>
      <button aria-label="Clear canvas" title="Clear canvas" onclick={clear}><Eraser size={16} /></button>
      <button aria-label="Light canvas" title="Light canvas" class:active={boardTheme === 'light'} onclick={() => setTheme('light')}><Sun size={16} /></button>
      <button aria-label="Dark canvas" title="Dark canvas" class:active={boardTheme === 'dark'} onclick={() => setTheme('dark')}><Moon size={16} /></button>
    </div>
  </header>
  <div class="quickdraw-host" bind:this={board}></div>
</section>

<style>
  .draw-panel { --ink: #10203a; --muted: #60708a; --line: rgba(16, 32, 58, .14); --paper: #f7f4ec; height: 100%; box-sizing: border-box; display: grid; grid-template-rows: auto 1fr; color: var(--ink); background: var(--paper); font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
  .draw-panel[data-theme='dark'] { --ink: #f4f5fb; --muted: #a7acc0; --line: rgba(244, 245, 251, .16); --paper: #17181c; }
  .draw-header { display: flex; align-items: end; justify-content: space-between; gap: 20px; padding: 22px clamp(20px, 4vw, 64px) 16px; border-bottom: 1px solid var(--line); }
  .draw-kicker { display: flex; align-items: center; gap: 7px; color: #1f2be0; font-size: 11px; font-weight: 700; letter-spacing: .13em; }
  h2 { margin: 10px 0 3px; font: 500 clamp(28px, 4vw, 48px) Georgia, serif; letter-spacing: -.05em; }
  p { margin: 0; color: var(--muted); font-size: 13px; }
  .draw-actions { display: flex; gap: 6px; }
  .draw-actions button { display: grid; place-items: center; width: 34px; height: 34px; border: 1px solid var(--line); border-radius: 7px; color: var(--ink); background: transparent; cursor: pointer; }
  .draw-actions button.active { color: white; background: #1f2be0; border-color: #1f2be0; }
  .quickdraw-host { min-height: 0; overflow: hidden; }
  :global(.draw-panel .qd-root), :global(.draw-panel .quickdraw) { height: 100%; }
</style>
