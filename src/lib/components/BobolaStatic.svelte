<!-- UNUSED. Replaced by BobolaSprite.svelte; nothing imports this file. Safe to delete (no delete tool was available). -->
<script lang="ts">
  let { size }: { size: number } = $props();
</script>

<svg viewBox="0 0 64 64" width={size} height={size} role="img" aria-label="Bobola, a calico cat">
  <ellipse cx="32" cy="55" rx="27" ry="7.5" fill="var(--cushion)" />
  <ellipse cx="32" cy="52" rx="24" ry="6" fill="var(--cushion-top)" />
  <path d="M12 47 Q5 38 13 33 Q12 41 21 47Z" fill="#e9913b" />
  <ellipse cx="32" cy="43" rx="17" ry="11" fill="#fbf0e1" />
  <path d="M39 33 Q51 36 49 46 Q44 52 36 52Z" fill="#2f2b33" />
  <path d="M15 44 Q18 34 26 33 Q24 44 28 52 Q18 52 15 44Z" fill="#e9913b" />
  <ellipse cx="32" cy="27" rx="15" ry="12.5" fill="#fbf0e1" />
  <path d="M17.5 21 L19 7 L30 15Z" fill="#e9913b" />
  <path d="M46.5 21 L45 7 L34 15Z" fill="#2f2b33" />
  <path d="M20.500 20 L21.500 12 L27 16Z" fill="#f4b9a4" />
  <path d="M43.500 20 L42.500 12 L37 16Z" fill="#f4b9a4" />
  <path d="M17 26 Q18 15 29 15 Q25 22 26 29 Q20 31 17 26Z" fill="#e9913b" />
  <path d="M47 24 Q46 16 39 16 Q42 22 41 27 Q45 28 47 24Z" fill="#2f2b33" />
  <g fill="none" stroke="#3b3340" stroke-width="1.600" stroke-linecap="round">
    <path d="M22 28 Q25 31 28 28" />
    <path d="M36 28 Q39 31 42 28" />
    <path d="M32 33 V34.500 M32 34.500 Q30 36 28 35.200 M32 34.500 Q34 36 36 35.200" stroke-width="1" />
  </g>
  <path d="M30.500 31.500 H33.500 L32 33.200Z" fill="#e58b8b" />
  <ellipse cx="25" cy="52" rx="5" ry="3" fill="#fbf0e1" />
  <ellipse cx="39" cy="52" rx="5" ry="3" fill="#fbf0e1" />
</svg>