export interface Service {
  id: string;
  name: string;
  url: string;
  workspaceId: string;
  profileName: string;
  protection: boolean;
  keepLive: boolean;
  color: string;
  idleMinutes?: number | null;
  catalogId?: string | null;
}
export interface Workspace {
  id: string;
  name: string;
}
export interface Preferences {
  liveBudget: number;
  idleMinutes: number;
  theme: 'light' | 'dark';
}
export interface Runtime {
  loaded: boolean;
  visible: boolean;
  dirty: boolean;
  media: boolean;
  blocked: number;
  lastActive: number;
  title: string;
  background?: [number, number, number] | null;
  lastUrl?: string | null;
}
export interface Snapshot {
  version: number;
  services: Service[];
  workspaces: Workspace[];
  preferences: Preferences;
}
export interface Bootstrap extends Snapshot {
  runtime: Record<string, Runtime>;
  filterRevision: string;
  native: boolean;
  /** Raw sidecar settings from the native backend. Validate with normalizeInterface. */
  interfacePreferences?: unknown;
}
export interface CatalogEntry {
  id: string;
  name: string;
  url: string;
  needsCustomUrl: boolean;
  license: string;
}
