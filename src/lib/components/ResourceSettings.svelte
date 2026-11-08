<script lang="ts">
  import { onMount } from 'svelte';
  import { command } from '../bridge';
  import type { Runtime, Service } from '../types';
  let {
    services,
    runtime,
    native,
    busy,
    onmanage,
    onsleep,
  }: {
    services: Service[];
    runtime: Record<string, Runtime>;
    native: boolean;
    busy: boolean;
    onmanage: (service: Service) => void;
    onsleep: (service: Service) => Promise<unknown>;
  } = $props();
  interface Sample {
    sampledAtMs: number;
    logicalProcessors: number;
    unavailable: number;
    processes: {
      pid: number;
      name: string;
      workingSetBytes: number | null;
      privateBytes: number | null;
      cpuSeconds: number | null;
      createdAt100ns: string | null;
    }[];
  }
  let sample = $state<Sample | null>(null),
    error = $state(''),
    cpu = $state<number | null>(null);
  const mib = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MiB`;
  async function refresh() {
    try {
      const next = await command<Sample>('resource_sample');
      if (sample && next.sampledAtMs > sample.sampledAtMs) {
        let seconds = 0;
        let matched = false;
        for (const process of next.processes) {
          const previous = sample.processes.find(
            (entry) =>
              entry.pid === process.pid &&
              entry.createdAt100ns !== null &&
              entry.createdAt100ns === process.createdAt100ns
          );
          if (
            process.cpuSeconds !== null &&
            previous?.cpuSeconds != null &&
            process.cpuSeconds >= previous.cpuSeconds
          ) {
            seconds += process.cpuSeconds - previous.cpuSeconds;
            matched = true;
          }
        }
        cpu = matched
          ? (seconds * 100000) / (next.sampledAtMs - sample.sampledAtMs) / next.logicalProcessors
          : null;
      }
      sample = next;
      error = '';
    } catch (failure) {
      error = String(failure);
    }
  }
  onMount(() => {
    if (!native) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const tick = async () => {
      if (!document.hidden) await refresh();
      if (!stopped) timer = setTimeout(tick, 3000);
    };
    void tick();
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  });
  const reason = (service: Service) => {
    const state = runtime[service.id];
    if (!state?.loaded) return 'Sleeping · opens when selected';
    if (state.visible) return 'Open on screen or in a popup';
    if (state.media) return 'Media or call in progress';
    if (state.dirty) return 'Unsaved work';
    if (service.keepLive) return 'Stays live for notifications';
    return 'Eligible to sleep when idle or over budget';
  };
</script>

<section class="settings-group" aria-label="Resource usage">
  <h3>Running accounts</h3>
  <p>
    {services.filter((service) => runtime[service.id]?.loaded).length} loaded of {services.length} saved.
    Sleeping accounts pause notifications and keep their sign-in on disk.
  </p>
  {#if sample}
    <div class="resource-totals">
      <strong
        >{mib(
          sample.processes.reduce((sum, process) => sum + (process.privateBytes ?? 0), 0)
        )}<small>Private memory</small></strong
      >
      <strong
        >{cpu === null ? 'Measuring…' : `${cpu.toFixed(1)}%`}<small
          >CPU estimate across logical processors</small
        ></strong
      >
    </div>
    <p>
      {sample.processes.length} owned processes, including browser children. Shared processes counted
      once. {sample.unavailable
        ? `${sample.unavailable} process measurements incomplete; totals are partial.`
        : ''}
    </p>
    <details>
      <summary>Process details</summary>
      <p>
        Working sets include shared pages and their sum is not unique physical RAM. Per-account
        memory is not attributed because browser processes can be shared. CPU compares matching
        process lifetimes between samples; work by processes that appear or exit between samples is
        omitted.
      </p>
      <div class="processes">
        <table>
          <thead><tr><th>Process</th><th>Private</th><th>Working set</th></tr></thead><tbody>
            {#each sample.processes as process (process.pid)}<tr
                ><td>{process.name}<small>PID {process.pid}</small></td><td
                  >{process.privateBytes === null ? 'Unavailable' : mib(process.privateBytes)}</td
                ><td
                  >{process.workingSetBytes === null
                    ? 'Unavailable'
                    : mib(process.workingSetBytes)}</td
                ></tr
              >{/each}
          </tbody>
        </table>
      </div>
    </details>
  {:else if !native}<p>Live process measurements are available in the Windows app.</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  {#each services as service (service.id)}
    <div class="setting-row">
      <div>
        <strong>{service.name}</strong>
        <p>{reason(service)}</p>
      </div>
      <div class="action-row">
        <button
          type="button"
          class="text-button"
          disabled={busy ||
            !native ||
            !runtime[service.id]?.loaded ||
            runtime[service.id]?.visible ||
            runtime[service.id]?.dirty ||
            runtime[service.id]?.media}
          onclick={() => onsleep(service)}>Sleep</button
        ><button type="button" class="text-button" onclick={() => onmanage(service)}>Manage</button>
      </div>
    </div>
  {/each}
</section>

<style>
  .resource-totals {
    display: flex;
    gap: 24px;
    margin: 16px 0;
    font-size: 20px;
  }
  small {
    display: block;
    font-size: 11px;
    font-weight: normal;
    color: var(--muted);
  }
  table {
    width: 100%;
    text-align: left;
    font-size: 12px;
    border-collapse: collapse;
  }
  td,
  th {
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
  }
  .processes {
    overflow: auto;
    max-height: 250px;
  }
  details {
    margin: 12px 0;
  }
</style>
