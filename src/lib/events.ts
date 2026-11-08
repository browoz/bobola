import { listen } from '@tauri-apps/api/event';
import type { WorkspaceEvent } from './interface-types';
import type { Runtime } from './types';

const MAX_ID = 128;
const MAX_MESSAGE = 240;
const MAX_TITLE = 300;
const MAX_RUNTIME_ENTRIES = 512;
const MAX_EXTRA = 16;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === 'object' && !Array.isArray(value);
const idOf = (value: unknown) =>
  typeof value === 'string' && value.length > 0 && value.length <= MAX_ID ? value : undefined;
const countOf = (value: unknown) =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : undefined;

/** Unrecognized fields, kept as inert data. The result has no prototype, so a `__proto__` key is just a key. */
function extrasOf(payload: Record<string, unknown>, known: readonly string[]) {
  const extra = Object.create(null) as Record<string, unknown>;
  let kept = 0;
  for (const key of Object.keys(payload)) {
    if (key === 'type' || known.includes(key)) continue;
    if (kept++ >= MAX_EXTRA) break;
    extra[key] = payload[key];
  }
  return extra;
}

function parseRuntime(value: unknown): Runtime | null {
  if (!isRecord(value)) return null;
  const { loaded, visible, dirty, media, title } = value;
  const blocked = countOf(value.blocked);
  const lastActive = countOf(value.lastActive);
  if (
    typeof loaded !== 'boolean' ||
    typeof visible !== 'boolean' ||
    typeof dirty !== 'boolean' ||
    typeof media !== 'boolean' ||
    typeof title !== 'string' ||
    blocked === undefined ||
    lastActive === undefined
  )
    return null;
  const background = Array.isArray(value.background) && value.background.length === 3 &&
    value.background.every((channel) => Number.isInteger(channel) && channel >= 0 && channel <= 255)
    ? value.background as [number, number, number] : null;
  return { loaded, visible, dirty, media, blocked, lastActive, title: title.slice(0, MAX_TITLE), background };
}

/** All or nothing: one invalid entry rejects the map so the caller can fall back to a refresh. */
function parseRuntimeMap(value: unknown): Record<string, Runtime> | null {
  if (!isRecord(value)) return null;
  const keys = Object.keys(value);
  if (keys.length > MAX_RUNTIME_ENTRIES) return null;
  const out: Record<string, Runtime> = {};
  for (const key of keys) {
    const entry = parseRuntime(value[key]);
    if (!entry || !idOf(key) || key === '__proto__') return null;
    out[key] = entry;
  }
  return out;
}

/** Accepts known tags only. Payload fields are data: never executed or rendered as HTML. */
export function parseWorkspaceEvent(payload: unknown): WorkspaceEvent | null {
  if (!isRecord(payload)) return null;
  const type = payload.type;
  switch (type) {
    case 'quickSwitchRequested':
      return {
        type,
        serviceId: idOf(payload.serviceId),
        extra: extrasOf(payload, ['serviceId']),
      };
    case 'popupOpened':
    case 'popupClosed':
      return {
        type,
        serviceId: idOf(payload.serviceId),
        popupId: countOf(payload.popupId),
        extra: extrasOf(payload, ['serviceId', 'popupId']),
      };
    case 'popupError':
      return {
        type,
        serviceId: idOf(payload.serviceId),
        message:
          typeof payload.message === 'string' ? payload.message.slice(0, MAX_MESSAGE) : undefined,
        extra: extrasOf(payload, ['serviceId', 'message']),
      };
    case 'windowVisibility':
      if (typeof payload.visible !== 'boolean') return null;
      return { type, visible: payload.visible, extra: extrasOf(payload, ['visible']) };
    case 'runtimeChanged': {
      const runtime = parseRuntimeMap(payload.runtime);
      return runtime ? { type, runtime, extra: extrasOf(payload, ['runtime']) } : null;
    }
    case 'snapshotChanged':
      return { type, extra: extrasOf(payload, []) };
    default:
      return null;
  }
}

export function subscribeWorkspaceEvents(handler: (event: WorkspaceEvent) => void) {
  return listen<unknown>('workspace-event', ({ payload }) => {
    const event = parseWorkspaceEvent(payload);
    if (event) handler(event);
    // A malformed runtime map must not leave counters stale: ask for a real refresh instead.
    else if (isRecord(payload) && payload.type === 'runtimeChanged')
      handler({ type: 'snapshotChanged', extra: Object.create(null) });
  });
}
