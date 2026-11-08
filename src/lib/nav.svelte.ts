import type { NavMode } from './interface-types';

export const HOVER_OPEN_MS = 120;
export const LEAVE_GRACE_MS = 250;

/**
 * Sidebar open state. Hover opens after 120 ms and closes 250 ms after the pointer leaves the
 * rail, cat and sidebar as one group. A click or keyboard reveal is sticky: pointer leave no
 * longer closes it, only Escape, outside press, window blur, selection or another toggle.
 * Nothing here waits for animation.
 */
export function createNav(mode: () => NavMode, hoverEnabled: () => boolean) {
  let open = $state(false);
  let sticky = false;
  let enterTimer: ReturnType<typeof setTimeout> | undefined;
  let leaveTimer: ReturnType<typeof setTimeout> | undefined;
  const clear = () => {
    clearTimeout(enterTimer);
    clearTimeout(leaveTimer);
    enterTimer = leaveTimer = undefined;
  };
  return {
    get open() {
      return mode() === 'pinned' || open;
    },
    pointerEnter() {
      clearTimeout(leaveTimer);
      leaveTimer = undefined;
      if (open || enterTimer !== undefined || !hoverEnabled()) return;
      enterTimer = setTimeout(() => {
        enterTimer = undefined;
        open = true;
      }, HOVER_OPEN_MS);
    },
    pointerLeave() {
      clearTimeout(enterTimer);
      enterTimer = undefined;
      if (!open || sticky) return;
      clearTimeout(leaveTimer);
      leaveTimer = setTimeout(() => {
        leaveTimer = undefined;
        open = false;
      }, LEAVE_GRACE_MS);
    },
    toggle() {
      clear();
      if (open && sticky) {
        open = false;
        sticky = false;
      } else {
        open = true;
        sticky = true;
      }
    },
    reveal() {
      clear();
      open = true;
      sticky = true;
    },
    dismiss() {
      clear();
      open = false;
      sticky = false;
    },
    destroy: clear,
  };
}