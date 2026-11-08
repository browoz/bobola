<script lang="ts">
  import type { Runtime, Service } from '../types';
  let { services, runtime, native, busy, onprotection }: {
    services: Service[];
    runtime: Record<string, Runtime>;
    native: boolean;
    busy: boolean;
    onprotection: (service: Service) => void;
  } = $props();
  const total = $derived(services.reduce((sum, service) => sum + (runtime[service.id]?.blocked ?? 0), 0));
</script>

<section class="settings-group" aria-label="Ad and tracker protection">
  <h3>Ad & tracker protection</h3>
  <p>Block ads and trackers across your accounts. Choose which services to protect below.</p>
  {#if native}
    <p class="protection-total"><strong>{total.toLocaleString()}</strong> network requests blocked across current accounts this app session.</p>
  {:else}
    <p>Live blocking statistics are available in the Windows app.</p>
  {/if}
  <details><summary>About these counts</summary><p>Counts cover blocked network requests this app session, not unique ads. Page elements hidden by cosmetic rules are not included.</p></details>
  <div class="account-list">
    {#each services as service (service.id)}
      <div class="setting-row">
        <div class="account-name"><strong>{service.name}</strong><p>{native ? `${(runtime[service.id]?.blocked ?? 0).toLocaleString()} requests blocked this session` : 'Preview account'}</p></div>
        <button type="button" class="toggle" role="switch" disabled={busy || !native}
          aria-label={`Protection for ${service.name}`} aria-checked={service.protection}
          onclick={() => onprotection(service)}><span></span></button>
      </div>
    {:else}
      <p>Add a service to configure its protection.</p>
    {/each}
  </div>
</section>

<style>
  .protection-total strong { font-size: 20px; font-variant-numeric: tabular-nums; color: var(--accent); }
  .account-list { margin-top: 12px; }
  details { margin-top: 12px; font-size: 12px; color: var(--muted); }
  summary { cursor: pointer; }
  .account-name { min-width: 0; overflow-wrap: anywhere; }
  .toggle { flex-shrink: 0; }
</style>
