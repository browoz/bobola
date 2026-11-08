import type { Runtime } from './types';

// Contract owned by the UI. The native coordinator implements matching sidecar settings;
// none of this belongs in the Rust `Preferences` struct (deny_unknown_fields).
export const NAV_MODES = ['minimal', 'autohide', 'pinned', 'glass-dock', 'orb-launcher'] as const;
export const DENSITIES = [40, 44, 48, 52] as const;
export const THEMES = ['system', 'light', 'dark'] as const;
export const ACCENTS = ['lavender', 'sky', 'sage', 'rose'] as const;
export const MOTIONS = ['system', 'reduced'] as const;
export const COMPANION_SIZES = [28, 34, 40] as const;
export const PRIVACY_LEVELS = ['hidden', 'sender', 'preview'] as const;
export const CLOSE_BEHAVIORS = ['quit', 'tray'] as const;

export type NavMode = (typeof NAV_MODES)[number];
export type Density = (typeof DENSITIES)[number];
export type ThemeChoice = (typeof THEMES)[number];
export type Accent = (typeof ACCENTS)[number];
export type MotionChoice = (typeof MOTIONS)[number];
export type CompanionSize = (typeof COMPANION_SIZES)[number];
export type NotificationPrivacy = (typeof PRIVACY_LEVELS)[number];
export type CloseBehavior = (typeof CLOSE_BEHAVIORS)[number];

export interface InterfacePreferences {
  version: 1;
  navMode: NavMode;
  density: Density;
  theme: ThemeChoice;
  accent: Accent;
  companion: { enabled: boolean; size: CompanionSize; motion: MotionChoice };
  /** from/to are local 24-hour HH:MM strings. */
  quiet: { enabled: boolean; hoursEnabled: boolean; from: string; to: string };
  notifications: { privacy: NotificationPrivacy };
  /** 'tray' is reserved; the UI keeps it disabled until the backend supports it. */
  closeBehavior: CloseBehavior;
}

export interface SurfaceRect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface SurfaceServiceRect extends SurfaceRect {
  serviceId: string;
}

/** Payload of the set_surface_layout command. `revision` is assigned by the surface adapter. */
export interface SurfaceLayout {
  revision: number;
  services: SurfaceServiceRect[];
  /** Shell UI drawn over website slots. The backend must keep these regions free of native views. */
  exclusions: SurfaceRect[];
  /** True while a modal dialog is open and website views must not take input. */
  modalInputCapture: boolean;
}
export type SurfaceCapability = 'unknown' | 'supported' | 'missing';

/**
 * Fields the parser did not recognize, kept as inert data (never rendered or executed) so a later
 * milestone can read them without changing the adapter. Capped in count; the object has no prototype.
 */
export interface EventExtras {
  extra: Record<string, unknown>;
}

/** Events on the `workspace-event` channel (camelCase tagged). Unknown tags are dropped. */
export type WorkspaceEvent = EventExtras &
  (
    | { type: 'quickSwitchRequested'; serviceId?: string }
    | { type: 'popupOpened'; serviceId?: string; popupId?: number }
    | { type: 'popupClosed'; serviceId?: string; popupId?: number }
    | { type: 'popupError'; serviceId?: string; message?: string }
    /** Backend forthcoming: shell window shown (true) or minimized/hidden (false). */
    | { type: 'windowVisibility'; visible: boolean }
    /** Backend forthcoming: complete runtime map, replaces the current one. */
    | { type: 'runtimeChanged'; runtime: Record<string, Runtime> }
    /** Backend forthcoming: the stored snapshot changed; refresh with bootstrap. */
    | { type: 'snapshotChanged' }
  );

/** Argument `command` of workspace_command. Only the variant the UI sends is typed. */
export type WorkspaceCommand = { type: 'focusService'; serviceId: string | null };
