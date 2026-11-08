<script lang="ts">
  import { getServiceIcon } from '../service-icons';

  let {
    name,
    url = '',
    size = 28,
    class: className = '',
    catalogId = null,
  }: {
    name: string;
    url?: string;
    size?: number;
    class?: string;
    catalogId?: string | null;
  } = $props();

  const iconUrl = $derived(getServiceIcon(name, url, catalogId));
  const initial = (str: string) => ([...(str || '').trim()][0] ?? '?').toUpperCase();
</script>

{#if iconUrl}
  <span
    class="service-logo {className}"
    style="width: {size}px; height: {size}px;"
    aria-hidden="true"
  >
    <img src={iconUrl} width={size} height={size} alt="" draggable="false" />
  </span>
{:else}
  <span
    class="avatar {className}"
    style={size !== 28
      ? `width: ${size}px; height: ${size}px; font-size: ${Math.round(size * 0.46)}px;`
      : undefined}
    aria-hidden="true"
  >
    {initial(name)}
  </span>
{/if}
