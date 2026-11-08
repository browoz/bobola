import type { FrameId } from './companion-frames';

/**
 * Deterministic Bobola state machine. Pure: no timers, no clock, no randomness.
 * The component owns one 100 ms interval that sends `tick` only while `isRunning(state)`.
 * Resting states (`sleep`, `sit`) hold their frame and need no timer at all. Nothing here wakes
 * Bobola on its own: only hover, focus, click or a verified cue start a sequence.
 */
export const TICK_MS = 100;

export type Mode =
  | 'sleep'
  | 'sit'
  | 'wake'
  | 'stretch'
  | 'walk'
  | 'blink'
  | 'notification'
  | 'returnToSleep';
type Timed = Exclude<Mode, 'sleep' | 'sit'>;

/** One entry per 100 ms tick. */
export const SEQUENCES: Record<Timed, readonly FrameId[]> = {
  wake: ['drowse', 'wake', 'wake'],
  stretch: ['stretch-1', 'stretch-1', 'stretch-2', 'stretch-2', 'stretch-2', 'stretch-1'],
  walk: ['walk-1', 'walk-2', 'walk-1', 'walk-2'],
  blink: ['blink', 'blink'],
  notification: ['alert-1', 'alert-2', 'alert-1', 'alert-2', 'alert-1', 'alert-2'],
  returnToSleep: ['sit', 'drowse', 'drowse'],
};

declare const verified: unique symbol;
/**
 * A request to play the notification sequence. The brand means only code that validated a typed
 * native event can produce one. No producer exists yet: the future notification adapter must
 * export the constructor. Nothing in the UI creates cues today.
 */
export interface VerifiedCue {
  readonly [verified]: true;
  /** Distinct per accepted native event, so a repeated prop value is not replayed. */
  readonly id: number;
}

export interface Fsm {
  mode: Mode;
  step: number;
  /** Pointer over, keyboard focus on, or context menu open. */
  hovered: boolean;
  /** A cue arrived mid-sequence; played once when the sequence ends. Further cues coalesce. */
  pendingCue: boolean;
  paused: boolean;
}

export type FsmEvent =
  | { type: 'hover' }
  | { type: 'leave' }
  | { type: 'click' }
  | { type: 'cue' }
  | { type: 'tick' }
  | { type: 'pause' }
  | { type: 'resume' };

export const REST: Fsm = {
  mode: 'sleep',
  step: 0,
  hovered: false,
  pendingCue: false,
  paused: false,
};

export const isRunning = (state: Fsm) =>
  !state.paused && state.mode !== 'sleep' && state.mode !== 'sit';

export function frameOf(state: Fsm): FrameId {
  if (state.mode === 'sleep') return 'sleep';
  if (state.mode === 'sit') return 'sit';
  const frames = SEQUENCES[state.mode];
  return frames[Math.min(state.step, frames.length - 1)];
}

const begin = (state: Fsm, mode: Mode): Fsm => ({ ...state, mode, step: 0 });

/** Next resting or cue state once a sequence has nothing else to chain. */
function settle(state: Fsm): Fsm {
  if (state.pendingCue) return { ...begin(state, 'notification'), pendingCue: false };
  return begin(state, state.hovered ? 'sit' : 'returnToSleep');
}

function finish(state: Fsm): Fsm {
  switch (state.mode) {
    case 'wake':
      return state.hovered ? begin(state, 'stretch') : settle(state);
    case 'stretch':
      return state.hovered ? begin(state, 'walk') : settle(state);
    case 'returnToSleep':
      return state.pendingCue || state.hovered ? settle(state) : begin(state, 'sleep');
    default:
      return settle(state);
  }
}

export function reduce(state: Fsm, event: FsmEvent): Fsm {
  if (event.type === 'pause') return { ...REST, paused: true };
  if (event.type === 'resume') return REST;
  if (state.paused) return state;
  switch (event.type) {
    case 'hover':
      if (state.mode === 'sleep' || state.mode === 'returnToSleep')
        return { ...begin(state, 'wake'), hovered: true };
      return state.hovered ? state : { ...state, hovered: true };
    case 'leave':
      if (!state.hovered) return state;
      if (state.mode === 'sit') return { ...begin(state, 'returnToSleep'), hovered: false };
      return { ...state, hovered: false };
    case 'click':
      if (state.mode === 'sleep' || state.mode === 'returnToSleep') return begin(state, 'wake');
      if (state.mode === 'sit') return begin(state, 'blink');
      return state;
    case 'cue':
      if (state.mode === 'sleep' || state.mode === 'sit') return begin(state, 'notification');
      if (state.mode === 'notification' || state.pendingCue) return state;
      return { ...state, pendingCue: true };
    case 'tick': {
      if (!isRunning(state)) return state;
      const step = state.step + 1;
      return step >= SEQUENCES[state.mode as Timed].length
        ? finish(state)
        : { ...state, step };
    }
  }
}
