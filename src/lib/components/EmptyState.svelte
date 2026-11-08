<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import ServiceIcon from './ServiceIcon.svelte';
  import type { Service } from '../types';

  let {
    services,
    hasAny,
    search = $bindable(''),
    onopen,
    onadd,
    onsuggest,
  }: {
    services: Service[];
    hasAny: boolean;
    search: string;
    onopen: (id: string) => void;
    onadd: () => void;
    onsuggest: (name: string, url: string) => void;
  } = $props();

  const suggestions = [
    { label: 'Gmail', name: 'Gmail', url: 'https://mail.google.com' },
    { label: 'Slack', name: 'Slack', url: 'https://app.slack.com' },
    { label: 'WhatsApp', name: 'WhatsApp', url: 'https://web.whatsapp.com' },
    { label: 'Custom website', name: '', url: '' },
  ];
  function host(url: string) {
    try {
      return new URL(url).hostname;
    } catch {
      return url;
    }
  }
</script>

<section class="empty" aria-label={hasAny ? 'Choose a service' : 'Get started'}>
  <h1>{hasAny ? 'Choose a service' : 'Add your first service'}</h1>
  {#if hasAny}
    <label class="inline-search"
      ><Search size={15} /><input
        aria-label="Filter services"
        placeholder="Find a service"
        bind:value={search}
      /></label
    >
    <div class="empty-list">
      {#each services as service (service.id)}
        <button type="button" onclick={() => onopen(service.id)}>
          <ServiceIcon name={service.name} url={service.url} catalogId={service.catalogId} />
          <span>{service.name}<small>{host(service.url)}</small></span>
        </button>
      {:else}
        <p>No matching services.</p>
      {/each}
    </div>
  {:else}
    <p>Each account keeps its own sign-in on this PC.</p>
    <div class="chips">
      {#each suggestions as item (item.label)}
        <button type="button" class="chip" onclick={() => onsuggest(item.name, item.url)}
          >{item.label}</button
        >
      {/each}
    </div>
  {/if}
  <button type="button" class="button primary" onclick={onadd}
    ><Plus size={16} />{hasAny ? 'Add another service' : 'Add service'}</button
  >
</section>
