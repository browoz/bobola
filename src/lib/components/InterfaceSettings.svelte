<script lang="ts">
  import Segmented from './Segmented.svelte';
  import type { InterfacePreferences } from '../interface-types';

  let {
    draft = $bindable(),
    show,
    native,
  }: { draft: InterfacePreferences; show: (text: string) => boolean; native: boolean } = $props();
</script>

{#snippet toggle(label: string, checked: boolean, change: (value: boolean) => void)}
  <button
    type="button"
    class="toggle"
    role="switch"
    aria-checked={checked}
    aria-label={label}
    onclick={() => change(!checked)}><span></span></button
  >
{/snippet}

{#if show('service navigation sidebar rail density auto-hide pinned minimal floating glass dock orb launcher')}
    <div class="setting-row launcher-setting">
      <div><strong>Service Navigation</strong><p>Choose a rail, sidebar, floating dock or orb launcher.</p></div>
      <select aria-label="Service Navigation" bind:value={draft.navMode}>
        <option value="minimal">Minimal Rail</option>
        <option value="autohide">Auto-hide</option>
        <option value="pinned">Pinned Sidebar</option>
        <option value="glass-dock">Floating Glass Dock</option>
        <option value="orb-launcher">Minimal Orb Launcher</option>
      </select>
    </div>
  <div class="setting-row">
    <div><strong>Rail size</strong><p>Width of the collapsed rail in pixels.</p></div>
    <Segmented
      label="Rail size"
      value={draft.density}
      options={[
        { value: 40, label: '40' },
        { value: 44, label: '44' },
        { value: 48, label: '48' },
        { value: 52, label: '52' },
      ]}
      onchange={(value) => (draft.density = value)}
    />
  </div>
{/if}

<style>
  .launcher-setting { flex-wrap: wrap; }
  .launcher-setting > div { flex: 1 1 200px; min-width: 0; }
  .launcher-setting select { max-width: 100%; min-width: 0; }
</style>
{#if show('appearance theme dark light accent color')}
  <div class="setting-row">
    <div><strong>Appearance</strong><p>Follow Windows or choose light or dark.</p></div>
    <Segmented
      label="Theme"
      value={draft.theme}
      options={[
        { value: 'system', label: 'System' },
        { value: 'light', label: 'Light' },
        { value: 'dark', label: 'Dark' },
      ]}
      onchange={(value) => (draft.theme = value)}
    />
  </div>
  <div class="setting-row">
    <div><strong>Accent</strong></div>
    <Segmented
      label="Accent color"
      value={draft.accent}
      options={[
        { value: 'lavender', label: 'Lavender' },
        { value: 'sky', label: 'Sky' },
        { value: 'sage', label: 'Sage' },
        { value: 'rose', label: 'Rose' },
      ]}
      onchange={(value) => (draft.accent = value)}
    />
  </div>
{/if}
{#if show('bobola companion cat size animation motion')}
  <div class="setting-row">
    <div><strong>Bobola</strong><p>The cat in the bottom-left corner of the navigation.</p></div>
    {@render toggle('Show Bobola', draft.companion.enabled, (value) => (draft.companion.enabled = value))}
  </div>
  <div class="setting-row">
    <div><strong>Bobola size</strong></div>
    <Segmented
      label="Bobola size"
      value={draft.companion.size}
      options={[
        { value: 28, label: 'Small' },
        { value: 34, label: 'Medium' },
        { value: 40, label: 'Large' },
      ]}
      onchange={(value) => (draft.companion.size = value)}
    />
  </div>
  <div class="setting-row">
    <div><strong>Animation</strong><p>Reduced stops nonessential motion. Windows reduced-motion is always respected.</p></div>
    <Segmented
      label="Animation"
      value={draft.companion.motion}
      options={[
        { value: 'system', label: 'System' },
        { value: 'reduced', label: 'Reduced' },
      ]}
      onchange={(value) => (draft.companion.motion = value)}
    />
  </div>
{/if}
{#if show('quiet mode hours notifications privacy do not disturb')}
  <div class="setting-row">
    <div><strong>Quiet mode</strong><p>Suppresses alerts and animation. Counts are still kept.</p></div>
    {@render toggle('Quiet mode', draft.quiet.enabled, (value) => (draft.quiet.enabled = value))}
  </div>
  <div class="setting-row">
    <div>
      <strong>Quiet hours</strong>
      <p>
        Quiet mode applies between these local times.
        <span class="hours"
          ><input type="time" aria-label="Quiet hours start" bind:value={draft.quiet.from} /> to
          <input type="time" aria-label="Quiet hours end" bind:value={draft.quiet.to} /></span
        >
      </p>
    </div>
    {@render toggle('Use quiet hours', draft.quiet.hoursEnabled, (value) => (draft.quiet.hoursEnabled = value))}
  </div>
  <div class="setting-row">
    <div>
      <strong>Notification privacy</strong>
      <p>Message content stays hidden by default.</p>
      {#if !native}<p>Notifications are available in the Windows app only.</p>{/if}
    </div>
    <Segmented
      label="Notification privacy"
      value={draft.notifications.privacy}
      options={[
        { value: 'hidden', label: 'Hidden' },
        { value: 'sender', label: 'Sender' },
        { value: 'preview', label: 'Preview' },
      ]}
      onchange={(value) => (draft.notifications.privacy = value)}
    />
  </div>
  <div class="setting-row">
    <div>
      <strong>When the window closes</strong>
      <p>Keep in tray needs native support that is not available yet.</p>
    </div>
    <Segmented
      label="Close behavior"
      value={draft.closeBehavior}
      options={[
        { value: 'quit', label: 'Quit' },
        { value: 'tray', label: 'Keep in tray', disabled: true, hint: 'Not available yet' },
      ]}
      onchange={(value) => (draft.closeBehavior = value)}
    />
  </div>
{/if}
