import { command } from './bridge';
import {
  ACCENTS,
  CLOSE_BEHAVIORS,
  COMPANION_SIZES,
  DENSITIES,
  MOTIONS,
  NAV_MODES,
  PRIVACY_LEVELS,
  THEMES,
} from './interface-types';
import type { InterfacePreferences } from './interface-types';

const defaults: InterfacePreferences = {
  version: 1,
  navMode: 'minimal',
  density: 48,
  theme: 'system',
  accent: 'lavender',
  companion: { enabled: true, size: 40, motion: 'system' },
  quiet: { enabled: false, hoursEnabled: false, from: '22:00', to: '07:00' },
  notifications: { privacy: 'hidden' },
  closeBehavior: 'quit',
};

function pick<T extends string | number>(value: unknown, allowed: readonly T[], fallback: T): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}
const flag = (value: unknown, fallback: boolean) =>
  typeof value === 'boolean' ? value : fallback;
const clock = (value: unknown, fallback: string) =>
  typeof value === 'string' && /^([01]\d|2[0-3]):[0-5]\d$/.test(value) ? value : fallback;
const record = (value: unknown): Record<string, unknown> =>
  value && typeof value === 'object' ? (value as Record<string, unknown>) : {};

/** Validates untrusted stored/native data. Without interface data, the legacy theme is kept. */
export function normalizeInterface(
  value: unknown,
  legacyTheme?: 'light' | 'dark'
): InterfacePreferences {
  const v = record(value);
  const companion = record(v.companion);
  const quiet = record(v.quiet);
  const notifications = record(v.notifications);
  return {
    version: 1,
    navMode: pick(v.navMode, NAV_MODES, defaults.navMode),
    density: pick(v.density, DENSITIES, defaults.density),
    theme: pick(v.theme, THEMES, legacyTheme ?? defaults.theme),
    accent: pick(v.accent, ACCENTS, defaults.accent),
    companion: {
      enabled: flag(companion.enabled, defaults.companion.enabled),
      size: pick(companion.size, COMPANION_SIZES, defaults.companion.size),
      motion: pick(companion.motion, MOTIONS, defaults.companion.motion),
    },
    quiet: {
      enabled: flag(quiet.enabled, defaults.quiet.enabled),
      hoursEnabled: flag(quiet.hoursEnabled, defaults.quiet.hoursEnabled),
      from: clock(quiet.from, defaults.quiet.from),
      to: clock(quiet.to, defaults.quiet.to),
    },
    notifications: {
      privacy: pick(notifications.privacy, PRIVACY_LEVELS, defaults.notifications.privacy),
    },
    closeBehavior: pick(v.closeBehavior, CLOSE_BEHAVIORS, defaults.closeBehavior),
  };
}

/** Real persistence: rejects when the backend command is missing or fails. */
export async function saveInterface(preferences: InterfacePreferences) {
  await command('save_interface_preferences', { preferences });
}