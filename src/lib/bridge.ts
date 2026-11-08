import { invoke } from '@tauri-apps/api/core';
import type { Bootstrap, Preferences, Service, Snapshot } from './types';

export const native = '__TAURI_INTERNALS__' in window;
const key = 'service-workspace-preview-v1';
function load(): Snapshot {
  try {
    const value = localStorage.getItem(key);
    if (value) return JSON.parse(value);
  } catch {
    /* Start with an empty preview. */
  }
  return {
    version: 1,
    services: [],
    workspaces: [{ id: 'personal', name: 'Personal' }],
    preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
  };
}
function save(data: Snapshot) {
  localStorage.setItem(key, JSON.stringify(data));
}

// Browser preview exercises the shell; remote service views exist only in the native app.
export async function command<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  if (native) return invoke<T>(name, args);
  const data = load();
  switch (name) {
    case 'bootstrap': {
      let interfacePreferences: unknown;
      try {
        interfacePreferences = JSON.parse(localStorage.getItem(`${key}-interface`) ?? 'null');
      } catch {
        /* Ignore unreadable preview data. */
      }
      return {
        ...data,
        runtime: {},
        filterRevision: '',
        native: false,
        interfacePreferences,
      } as T;
    }
    case 'add_service': {
      const input = args.input as Pick<Service, 'name' | 'url' | 'workspaceId'> &
        Partial<Pick<Service, 'keepLive' | 'catalogId'>>;
      const parsed = new URL(input.url);
      if (
        !['http:', 'https:'].includes(parsed.protocol) ||
        parsed.username ||
        parsed.password ||
        !input.name.trim()
      )
        throw new Error('Use an HTTP or HTTPS website and a service name.');
      const id = crypto.randomUUID().replaceAll('-', '');
      const service: Service = {
        ...input,
        name: input.name.trim(),
        url: parsed.href,
        id,
        profileName: `account_${id}`,
        keepLive: input.keepLive ?? false,
        idleMinutes: null,
        protection: true,
        color: 'blue',
      };
      data.services.push(service);
      save(data);
      return service as T;
    }
    case 'update_service':
      data.services = data.services.map((service) =>
        service.id === (args.service as Service).id ? (args.service as Service) : service
      );
      save(data);
      break;
    case 'delete_service':
      data.services = data.services.filter((service) => service.id !== args.id);
      save(data);
      break;
    case 'add_workspace': {
      const workspace = {
        id: crypto.randomUUID().replaceAll('-', ''),
        name: String(args.name).trim(),
      };
      data.workspaces.push(workspace);
      save(data);
      return workspace as T;
    }
    case 'save_preferences':
      data.preferences = args.preferences as Preferences;
      save(data);
      break;
    // Browser preview only. In the native app these invoke the real backend and may reject.
    case 'save_interface_preferences':
      localStorage.setItem(`${key}-interface`, JSON.stringify(args.preferences));
      break;
    case 'set_surface_layout':
      break;
    case 'export_backup':
      return JSON.stringify(data, null, 2) as T;
    case 'set_layout':
    case 'unload_service':
    case 'reload_service':
      break;
    default:
      throw new Error('This action is available in the Windows desktop app.');
  }
  return undefined as T;
}

export const bootstrap = () => command<Bootstrap>('bootstrap');

/** Native keyboard focus. `null` restores the backend's last focused visible service. */
export const focusService = (serviceId: string | null) =>
  command('workspace_command', { command: { type: 'focusService', serviceId } });

export type MusicStatus = {
  available: boolean;
  running: boolean;
  title?: string | null;
  artist?: string | null;
  paused?: boolean;
  volume?: number;
  error?: string | null;
};

export const musicStatus = () => command<MusicStatus>('music_status');
export const musicCommand = (action: 'play' | 'pause' | 'next' | 'previous' | 'volumeUp' | 'volumeDown', query?: string) =>
  command<MusicStatus>('music_command', { action, query });
