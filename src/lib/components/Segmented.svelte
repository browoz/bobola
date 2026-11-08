<script lang="ts" generics="T extends string | number">
  let {
    label,
    value,
    options,
    onchange,
  }: {
    label: string;
    value: T;
    options: { value: T; label: string; disabled?: boolean; hint?: string }[];
    onchange: (value: T) => void;
  } = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === option.value}
      disabled={option.disabled}
      title={option.hint}
      onclick={() => onchange(option.value)}>{option.label}</button
    >
  {/each}
</div>