<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Music2 from '@lucide/svelte/icons/music-2';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import TimerReset from '@lucide/svelte/icons/timer-reset';

  let { onclose = () => {} }: { onclose?: () => void } = $props();

  type TimerMode = 'focus' | 'pomodoro';
  let mode = $state<TimerMode>('focus');
  let pomodoroPhase = $state<'focus' | 'break'>('focus');
  let remaining = $state(25 * 60);
  let running = $state(false);
  let infinite = $state(false);
  let completed = $state(0);

  const total = $derived(mode === 'focus' ? 25 * 60 : pomodoroPhase === 'focus' ? 25 * 60 : 5 * 60);
  const progress = $derived(Math.max(0, Math.min(1, 1 - remaining / total)));
  const time = $derived(
    `${String(Math.floor(remaining / 60)).padStart(2, '0')}:${String(remaining % 60).padStart(2, '0')}`
  );
  const label = $derived(
    mode === 'focus'
      ? running
        ? 'In focus'
        : 'Ready?'
      : pomodoroPhase === 'focus'
        ? 'Focus'
        : 'Break'
  );

  function reset() {
    running = false;
    remaining = total;
  }

  function selectMode(next: TimerMode) {
    mode = next;
    pomodoroPhase = 'focus';
    remaining = next === 'focus' ? 25 * 60 : 25 * 60;
    running = false;
  }

  function selectPhase(next: 'focus' | 'break') {
    pomodoroPhase = next;
    remaining = next === 'focus' ? 25 * 60 : 5 * 60;
    running = false;
  }

  function tick() {
    if (!running || infinite) return;
    if (remaining > 0) {
      remaining -= 1;
      return;
    }
    completed += 1;
    if (mode === 'pomodoro' && pomodoroPhase === 'focus') {
      selectPhase('break');
      running = true;
    } else {
      running = false;
      reset();
    }
  }

  $effect(() => {
    const interval = window.setInterval(tick, 1000);
    return () => window.clearInterval(interval);
  });
</script>

<section class="concentration-popover" aria-label="Concentration" data-exclusion>
  <header class="concentration-header">
    <div>
      <strong>Concentration</strong>
      <span
        >{completed
          ? `${completed} session${completed === 1 ? '' : 's'} today`
          : 'A quieter place to work'}</span
      >
    </div>
    <div class="concentration-tools">
      <button
        class="concentration-icon"
        type="button"
        aria-label="Sound settings"
        title="Sound settings"
      >
        <Music2 size={15} />
      </button>
      <button
        class="concentration-icon"
        type="button"
        aria-label="Timer settings"
        title="Timer settings"
      >
        <Settings2 size={15} />
      </button>
      <button
        class="concentration-icon"
        type="button"
        aria-label="Expand concentration"
        title="Expand concentration"
      >
        <Maximize2 size={15} />
      </button>
    </div>
  </header>

  <div class="concentration-tabs" role="tablist" aria-label="Concentration mode">
    <button
      type="button"
      role="tab"
      aria-selected={mode === 'focus'}
      class:active={mode === 'focus'}
      onclick={() => selectMode('focus')}>Focus</button
    >
    <button
      type="button"
      role="tab"
      aria-selected={mode === 'pomodoro'}
      class:active={mode === 'pomodoro'}
      onclick={() => selectMode('pomodoro')}>Pomodoro <span class="pro-badge">PRO</span></button
    >
  </div>

  {#if mode === 'pomodoro'}
    <div class="phase-picker" role="group" aria-label="Pomodoro phase">
      <button
        class:active={pomodoroPhase === 'focus'}
        type="button"
        onclick={() => selectPhase('focus')}>Focus</button
      >
      <button
        class:active={pomodoroPhase === 'break'}
        type="button"
        onclick={() => selectPhase('break')}>Break</button
      >
    </div>
  {/if}

  <div class="timer-ring" style={`--timer-progress: ${progress * 360}deg`}>
    <div class="timer-core">
      <strong>{time}</strong>
      <span>{label}</span>
    </div>
  </div>

  <label class="infinite-toggle">
    <input type="checkbox" bind:checked={infinite} />
    <span class="toggle-track"><span></span></span>
    <TimerReset size={14} />
    <span>Infinite mode</span>
    <ChevronDown size={13} />
  </label>

  <div class="concentration-actions">
    <button class="concentration-icon sound-button" type="button" aria-label="Choose ambient sound">
      <Music2 size={16} />
    </button>
    <button
      class="button primary concentration-start"
      type="button"
      onclick={() => (running = !running)}
    >
      {#if running}<Pause size={15} />Pause{:else}<Play size={15} />Start{/if}
    </button>
    <button
      class="concentration-icon"
      type="button"
      aria-label="Reset timer"
      title="Reset timer"
      onclick={reset}
    >
      <RotateCcw size={15} />
    </button>
  </div>

  {#if running}
    <p class="concentration-status">
      <Check size={13} /> Stay with the current account until the timer ends.
    </p>
  {:else}
    <p class="concentration-status">
      Use the timer to protect a small block of uninterrupted attention.
    </p>
  {/if}

  <button class="close-concentration" type="button" onclick={onclose}>Close concentration</button>
</section>
