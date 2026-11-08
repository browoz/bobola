/**
 * Frame metadata for the Bobola sprite atlas. The artwork lives in components/BobolaSprite.svelte
 * as one set of reusable SVG groups (poses, heads, eyes). A frame is a typed combination of those
 * groups plus a head transform, so every frame reuses the same shapes.
 */
export const CELL = 64;
/** Largest whole-figure nudge in cell units. Keeps all motion inside the dock. */
export const MAX_SHIFT = 2;

export const POSES = [
  'curled',
  'sit',
  'stretch-a',
  'stretch-b',
  'walk-a',
  'walk-b',
  'alert',
] as const;
export const EYES = ['closed', 'half', 'open', 'wide'] as const;
export const FRAME_IDS = [
  'sleep',
  'drowse',
  'wake',
  'stretch-1',
  'stretch-2',
  'walk-1',
  'walk-2',
  'sit',
  'blink',
  'alert-1',
  'alert-2',
] as const;

export type Pose = (typeof POSES)[number];
export type Eyes = (typeof EYES)[number];
export type FrameId = (typeof FRAME_IDS)[number];

export interface FrameMeta {
  pose: Pose;
  eyes: Eyes;
  /** Head centre offset from the sitting head (cell 32,27), rotation in degrees, and scale. */
  head: { x: number; y: number; rotate: number; scale: number };
  /** Whole-figure nudge in cell units, within +/- MAX_SHIFT. */
  shift: { x: number; y: number };
}

const head = (x = 0, y = 0, rotate = 0, scale = 1) => ({ x, y, rotate, scale });
const shift = (x = 0, y = 0) => ({ x, y });

export const FRAMES: Record<FrameId, FrameMeta> = {
  sleep: { pose: 'curled', eyes: 'closed', head: head(-2, 12, -8, 0.85), shift: shift() },
  drowse: { pose: 'curled', eyes: 'half', head: head(-2, 11, -4, 0.85), shift: shift() },
  wake: { pose: 'curled', eyes: 'open', head: head(-2, 9, 0, 0.88), shift: shift() },
  'stretch-1': { pose: 'stretch-a', eyes: 'half', head: head(-9, 12, -14, 0.9), shift: shift(0, 1) },
  'stretch-2': { pose: 'stretch-b', eyes: 'closed', head: head(-10, 13, -20, 0.9), shift: shift(0, 1) },
  'walk-1': { pose: 'walk-a', eyes: 'open', head: head(-6, 8, -4, 0.82), shift: shift(-1, 0) },
  'walk-2': { pose: 'walk-b', eyes: 'open', head: head(-6, 9, 4, 0.82), shift: shift(1, 1) },
  sit: { pose: 'sit', eyes: 'open', head: head(), shift: shift() },
  blink: { pose: 'sit', eyes: 'closed', head: head(0, 1, 0), shift: shift() },
  'alert-1': { pose: 'alert', eyes: 'wide', head: head(0, -1, -6), shift: shift() },
  'alert-2': { pose: 'alert', eyes: 'wide', head: head(0, -1, 6), shift: shift() },
};

/** True when a frame's offsets stay inside the dock cell. For tests or tooling; not called at runtime. */
export function frameInBounds(meta: FrameMeta) {
  return (
    Math.abs(meta.shift.x) <= MAX_SHIFT &&
    Math.abs(meta.shift.y) <= MAX_SHIFT &&
    Math.abs(meta.head.x) <= 12 &&
    Math.abs(meta.head.y) <= 14 &&
    meta.head.scale > 0.5 &&
    meta.head.scale <= 1
  );
}
