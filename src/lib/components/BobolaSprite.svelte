<!--
  Bobola sprite atlas: original calico cat (cream, orange, dark) on a lavender cushion.
  All art is reusable <g> groups in one <defs>; a frame (companion-frames.ts) picks a pose, an eye
  style and a head transform. The element ids are global to the document, so mount one instance.
  The shapes are reused from the previous static art. Frames are hand-written and not yet seen
  rendered.
-->
<script lang="ts">
  import { FRAMES, type FrameId } from '../companion-frames';

  let { size, frame }: { size: number; frame: FrameId } = $props();

  const meta = $derived(FRAMES[frame]);
  // The shared head is drawn around the sitting head centre (32,27).
  const headTransform = $derived(
    `translate(${32 + meta.head.x} ${27 + meta.head.y}) rotate(${meta.head.rotate}) ` +
      `scale(${meta.head.scale}) translate(-32 -27)`
  );
</script>

<svg
  class="bobola"
  viewBox="0 0 64 64"
  width={size}
  height={size}
  aria-hidden="true"
  focusable="false"
  data-frame={frame}
>
  <defs>
    <g id="bobola-cushion">
      <ellipse cx="32" cy="55" rx="27" ry="7.5" fill="var(--cushion)" />
      <ellipse cx="32" cy="52" rx="24" ry="6" fill="var(--cushion-top)" />
    </g>
    <g id="bobola-head">
      <ellipse cx="32" cy="27" rx="15" ry="12.5" fill="#fbf0e1" />
      <path d="M17.5 21 L19 7 L30 15Z" fill="#e9913b" />
      <path d="M46.5 21 L45 7 L34 15Z" fill="#2f2b33" />
      <path d="M20.5 20 L21.5 12 L27 16Z" fill="#f4b9a4" />
      <path d="M43.5 20 L42.5 12 L37 16Z" fill="#f4b9a4" />
      <path d="M17 26 Q18 15 29 15 Q25 22 26 29 Q20 31 17 26Z" fill="#e9913b" />
      <path d="M47 24 Q46 16 39 16 Q42 22 41 27 Q45 28 47 24Z" fill="#2f2b33" />
      <path d="M30.5 31.5 H33.5 L32 33.2Z" fill="#e58b8b" />
      <path
        d="M32 33 V34.5 M32 34.5 Q30 36 28 35.2 M32 34.5 Q34 36 36 35.2"
        fill="none"
        stroke="#3b3340"
        stroke-width="1"
        stroke-linecap="round"
      />
    </g>
    <g id="bobola-eyes-closed" fill="none" stroke="#3b3340" stroke-width="1.6" stroke-linecap="round">
      <path d="M22 28 Q25 31 28 28" />
      <path d="M36 28 Q39 31 42 28" />
    </g>
    <g id="bobola-eyes-half" fill="none" stroke="#3b3340" stroke-width="1.6" stroke-linecap="round">
      <path d="M22 27.6 H28" />
      <path d="M36 27.6 H42" />
      <path d="M23.2 29.6 Q25 30.6 26.8 29.6" stroke-width="1.2" />
      <path d="M37.2 29.6 Q39 30.6 40.8 29.6" stroke-width="1.2" />
    </g>
    <g id="bobola-eyes-open" fill="#2b2530">
      <ellipse cx="25" cy="28" rx="1.8" ry="2.2" />
      <ellipse cx="39" cy="28" rx="1.8" ry="2.2" />
    </g>
    <g id="bobola-eyes-wide">
      <ellipse cx="25" cy="28" rx="2.4" ry="2.9" fill="#2b2530" />
      <ellipse cx="39" cy="28" rx="2.4" ry="2.9" fill="#2b2530" />
      <circle cx="25.8" cy="27" r="0.8" fill="#fff" />
      <circle cx="39.8" cy="27" r="0.8" fill="#fff" />
    </g>
    <g id="bobola-pose-sit">
      <path d="M12 47 Q5 38 13 33 Q12 41 21 47Z" fill="#e9913b" />
      <ellipse cx="32" cy="43" rx="17" ry="11" fill="#fbf0e1" />
      <path d="M39 33 Q51 36 49 46 Q44 52 36 52Z" fill="#2f2b33" />
      <path d="M15 44 Q18 34 26 33 Q24 44 28 52 Q18 52 15 44Z" fill="#e9913b" />
      <ellipse cx="25" cy="52" rx="5" ry="3" fill="#fbf0e1" />
      <ellipse cx="39" cy="52" rx="5" ry="3" fill="#fbf0e1" />
    </g>
    <g id="bobola-pose-curled">
      <ellipse cx="32" cy="46" rx="20" ry="9" fill="#fbf0e1" />
      <path d="M36 38 Q50 40 51 47 Q46 54 34 54Z" fill="#2f2b33" />
      <path d="M13 46 Q15 38 25 38 Q23 46 26 54 Q16 53 13 46Z" fill="#e9913b" />
      <path d="M12 50 Q8 44 14 41" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
    </g>
    <g id="bobola-pose-stretch-a">
      <path d="M50 38 Q58 30 54 22" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
      <g transform="rotate(-10 32 46)">
        <ellipse cx="34" cy="44" rx="18" ry="9" fill="#fbf0e1" />
        <path d="M40 36 Q52 38 51 46 Q46 52 38 52Z" fill="#2f2b33" />
        <path d="M18 44 Q21 36 28 36 Q26 44 29 52 Q21 51 18 44Z" fill="#e9913b" />
      </g>
      <ellipse cx="14" cy="52" rx="7" ry="2.6" fill="#fbf0e1" />
      <ellipse cx="22" cy="53" rx="6" ry="2.6" fill="#fbf0e1" />
    </g>
    <g id="bobola-pose-stretch-b">
      <path d="M50 36 Q60 28 55 18" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
      <g transform="rotate(-16 32 46)">
        <ellipse cx="34" cy="43" rx="18" ry="9" fill="#fbf0e1" />
        <path d="M40 35 Q52 37 51 45 Q46 51 38 51Z" fill="#2f2b33" />
        <path d="M18 43 Q21 35 28 35 Q26 43 29 51 Q21 50 18 43Z" fill="#e9913b" />
      </g>
      <ellipse cx="11" cy="53" rx="8" ry="2.6" fill="#fbf0e1" />
      <ellipse cx="20" cy="54" rx="7" ry="2.6" fill="#fbf0e1" />
    </g>
    <g id="bobola-pose-walk-a">
      <path d="M50 42 Q58 38 56 30" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
      <ellipse cx="34" cy="43" rx="18" ry="8.5" fill="#fbf0e1" />
      <path d="M40 36 Q52 38 51 46 Q46 51 38 51Z" fill="#2f2b33" />
      <path d="M18 43 Q21 36 28 36 Q26 44 29 51 Q21 50 18 43Z" fill="#e9913b" />
      <ellipse cx="22" cy="52" rx="4" ry="3" fill="#fbf0e1" />
      <ellipse cx="44" cy="51" rx="4" ry="2.4" fill="#fbf0e1" />
    </g>
    <g id="bobola-pose-walk-b">
      <path d="M50 41 Q59 39 58 31" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
      <ellipse cx="34" cy="43" rx="18" ry="8.5" fill="#fbf0e1" />
      <path d="M40 36 Q52 38 51 46 Q46 51 38 51Z" fill="#2f2b33" />
      <path d="M18 43 Q21 36 28 36 Q26 44 29 51 Q21 50 18 43Z" fill="#e9913b" />
      <ellipse cx="28" cy="51" rx="4" ry="2.4" fill="#fbf0e1" />
      <ellipse cx="40" cy="52" rx="4" ry="3" fill="#fbf0e1" />
    </g>
    <g id="bobola-pose-alert">
      <path d="M12 47 Q4 36 12 28" fill="none" stroke="#e9913b" stroke-width="4" stroke-linecap="round" />
      <ellipse cx="32" cy="43" rx="17" ry="11" fill="#fbf0e1" />
      <path d="M39 33 Q51 36 49 46 Q44 52 36 52Z" fill="#2f2b33" />
      <path d="M15 44 Q18 34 26 33 Q24 44 28 52 Q18 52 15 44Z" fill="#e9913b" />
      <ellipse cx="25" cy="52" rx="5" ry="3" fill="#fbf0e1" />
      <ellipse cx="39" cy="52" rx="5" ry="3" fill="#fbf0e1" />
      <ellipse cx="46" cy="40" rx="3" ry="4.5" fill="#fbf0e1" stroke="#2f2b33" stroke-width="0.8" />
    </g>
  </defs>
  <use href="#bobola-cushion" />
  <g transform="translate({meta.shift.x} {meta.shift.y})">
    <use href="#bobola-pose-{meta.pose}" />
    <g transform={headTransform}>
      <use href="#bobola-head" />
      <use href="#bobola-eyes-{meta.eyes}" />
    </g>
  </g>
</svg>

<style>
  .bobola {
    display: block;
    overflow: hidden;
    contain: paint;
  }
</style>
