<script lang="ts">
  import Music2 from '@lucide/svelte/icons/music-2';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import SkipBack from '@lucide/svelte/icons/skip-back';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import { musicCommand, musicStatus, type MusicStatus } from '../bridge';

  let { theme = 'light' }: { theme?: 'light' | 'dark' } = $props();
  let status = $state<MusicStatus>({ available: false, running: false });
  let query = $state('');
  let busy = $state(false);
  let loading = $state(true);

  async function refresh() {
    try {
      status = await musicStatus();
    } catch (error) {
      status = { ...status, error: error instanceof Error ? error.message : String(error) };
    } finally {
      loading = false;
    }
  }
  async function run(action: Parameters<typeof musicCommand>[0], value?: string) {
    busy = true;
    try {
      status = await musicCommand(action, value);
    } catch (error) {
      status = { ...status, error: error instanceof Error ? error.message : String(error) };
    } finally {
      busy = false;
    }
  }
  async function search(event: SubmitEvent) {
    event.preventDefault();
    if (query.trim()) await run('play', query.trim());
  }
  $effect(() => {
    theme;
    void refresh();
    const timer = window.setInterval(refresh, 4000);
    return () => window.clearInterval(timer);
  });
</script>

<section class="music-panel" data-theme={theme} aria-label="Bobola Music">
  <header class="music-header">
    <div class="music-kicker"><Music2 size={14} /> MUSIC / YUTUTUI</div>
    <h2>Play something.</h2>
    <p>Search YouTube Music without opening another workspace.</p>
  </header>

  <form class="music-search" onsubmit={search}>
    <input bind:value={query} placeholder="Search a song, artist, or mix" aria-label="Search music" />
    <button class="music-action primary" disabled={busy || !query.trim()}>Play</button>
  </form>

  <div class="music-now">
    <span class="music-art"><Music2 size={22} /></span>
    <div class="music-track">
      <strong>{status.title || 'Nothing playing'}</strong>
      <span>{status.artist || (status.available ? 'Search to start playback' : 'YuTuTui is not installed')}</span>
    </div>
  </div>

  <div class="music-controls">
    <button class="music-icon" aria-label="Previous track" title="Previous" onclick={() => run('previous')} disabled={!status.running}><SkipBack size={17} /></button>
    <button class="music-play" aria-label={status.paused === false ? 'Pause' : 'Play'} onclick={() => run(status.paused === false ? 'pause' : 'play')} disabled={!status.running}>
      {#if status.paused === false}<Pause size={18} />{:else}<Play size={18} />{/if}
    </button>
    <button class="music-icon" aria-label="Next track" title="Next" onclick={() => run('next')} disabled={!status.running}><SkipForward size={17} /></button>
    <button class="music-icon" aria-label="Volume up" title="Volume up" onclick={() => run('volumeUp')} disabled={!status.running}><Volume2 size={17} /></button>
  </div>

  {#if loading}
    <div class="music-help" role="status">Starting the local music controller…</div>
  {:else if !status.available}
     <div class="music-help">The optional <a href="https://github.com/Ochichan/Yututui" target="_blank" rel="noreferrer">YuTuTui</a> companion is not installed in this build.</div>
  {:else if status.error}
    <div class="music-help error" role="alert">
      <span>{status.error}</span>
      <button type="button" class="retry" onclick={refresh} disabled={busy}>Retry connection</button>
    </div>
  {:else}
    <div class="music-help">Powered by <a href="https://github.com/Ochichan/Yututui" target="_blank" rel="noreferrer">YuTuTui</a> · local playback control</div>
  {/if}
</section>

<style>
  .music-panel { --ink: #10203a; --muted: #60708a; --line: rgba(16, 32, 58, .14); --paper: #f7f4ec; --cobalt: #1f2be0; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; height: 100%; padding: 26px clamp(20px, 4vw, 64px); color: var(--ink); background: var(--paper); font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
  .music-panel > * { width: min(100%, 760px); }
  .music-panel[data-theme='dark'] { --ink: #f4f5fb; --muted: #a7acc0; --line: rgba(244, 245, 251, .16); --paper: #17181c; --cobalt: #9ea5ff; }
  .music-header { max-width: 620px; padding-bottom: 28px; border-bottom: 1px solid var(--line); }
  .music-kicker { display: flex; align-items: center; gap: 8px; color: var(--cobalt); font-size: 11px; font-weight: 700; letter-spacing: .14em; }
  h2 { margin: 18px 0 8px; font-family: Georgia, serif; font-size: clamp(32px, 5vw, 64px); font-weight: 500; letter-spacing: -.05em; }
  p { margin: 0; color: var(--muted); font-size: 14px; }
  .music-search { display: flex; max-width: 620px; gap: 8px; margin: 28px 0; }
  input { width: 100%; min-width: 0; padding: 13px 14px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink); background: transparent; outline: none; }
  input:focus { border-color: var(--cobalt); box-shadow: 0 0 0 3px color-mix(in srgb, var(--cobalt) 18%, transparent); }
  button { color: inherit; font: inherit; cursor: pointer; }
  button:disabled { opacity: .45; cursor: default; }
  .music-action { border: 0; border-radius: 8px; padding: 0 20px; font-size: 13px; font-weight: 700; }
  .music-action.primary { color: white; background: var(--cobalt); }
  .music-now { display: flex; align-items: center; max-width: 620px; gap: 16px; padding: 20px 0; border-bottom: 1px solid var(--line); }
  .music-art { display: grid; place-items: center; width: 58px; height: 58px; border-radius: 10px; color: var(--cobalt); background: color-mix(in srgb, var(--cobalt) 14%, transparent); }
  .music-track { min-width: 0; display: grid; gap: 4px; }
  .music-track strong, .music-track span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .music-track span { color: var(--muted); font-size: 13px; }
  .music-controls { display: flex; align-items: center; gap: 10px; padding: 20px 0; }
  .music-icon, .music-play { display: grid; place-items: center; border: 1px solid var(--line); background: transparent; }
  .music-icon { width: 34px; height: 34px; border-radius: 50%; }
  .music-play { width: 44px; height: 44px; border-radius: 50%; color: white; background: var(--cobalt); border-color: var(--cobalt); }
  .music-help { display: flex; align-items: center; justify-content: space-between; gap: 16px; max-width: 760px; padding: 12px 0; color: var(--muted); font-size: 12px; line-height: 1.5; }
  .music-help a { color: var(--cobalt); }
  .music-help.error { color: #c23c3c; }
  .retry { flex: 0 0 auto; border: 1px solid currentColor; border-radius: 6px; padding: 7px 10px; color: inherit; background: transparent; font-size: 12px; }
  @media (max-width: 560px) { .music-help { align-items: flex-start; flex-direction: column; } }
</style>
