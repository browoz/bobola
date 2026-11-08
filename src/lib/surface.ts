import { command } from './bridge';
import type { SurfaceCapability, SurfaceLayout, SurfaceRect } from './interface-types';

type SurfaceInput = Omit<SurfaceLayout, 'revision'>;

let lastRevision = 0;
/** Starts at Date.now() so a shell reload cannot fall below a revision the backend already applied. */
const nextRevision = () => (lastRevision = Math.max(Date.now(), lastRevision + 1));

const rectOf = (element: Element): SurfaceRect => {
  const { x, y, width, height } = element.getBoundingClientRect();
  return { x, y, width, height };
};
const visible = (rect: SurfaceRect) => rect.width > 0 && rect.height > 0;

/**
 * Floating UI rectangle as the native view must leave it free: skipped when not rendered, clipped
 * to the client area (the Bobola menu can extend past it), and rounded outward so no sub-pixel
 * sliver of a website view shows at the edge.
 */
function exclusionOf(element: Element): SurfaceRect | null {
  if (getComputedStyle(element).visibility === 'hidden') return null;
  const box = element.getBoundingClientRect();
  const left = Math.max(0, Math.floor(box.left));
  const top = Math.max(0, Math.floor(box.top));
  const right = Math.min(document.documentElement.clientWidth, Math.ceil(box.right));
  const bottom = Math.min(document.documentElement.clientHeight, Math.ceil(box.bottom));
  const rect = { x: left, y: top, width: right - left, height: bottom - top };
  return visible(rect) ? rect : null;
}

/**
 * Reads website slots and floating UI (elements marked data-exclusion) from the DOM.
 * While a modal holds input there are no website slots to carve, so no exclusions are sent.
 */
export function measureSurface(modalOpen: boolean): SurfaceInput {
  if (modalOpen) return { services: [], exclusions: [], modalInputCapture: true };
  const services = [...document.querySelectorAll<HTMLElement>('[data-service-id]')]
    .map((element) => ({ serviceId: element.dataset.serviceId ?? '', ...rectOf(element) }))
    .filter((slot) => slot.serviceId && visible(slot));
  const exclusions = [...document.querySelectorAll('[data-exclusion]')]
    .map(exclusionOf)
    .filter((rect): rect is SurfaceRect => rect !== null);
  return { services, exclusions, modalInputCapture: false };
}

const missing = (failure: unknown) => {
  const text = failure instanceof Error ? failure.message : String(failure);
  return text.includes('set_surface_layout') && /not found|not allowed|unknown/i.test(text);
};

export function createSurface() {
  let capability: SurfaceCapability = 'unknown';
  let queue: Promise<void> = Promise.resolve();
  let latest = 0;

  async function send(input: SurfaceInput) {
    if (capability !== 'missing') {
      try {
        const layout = { revision: nextRevision(), ...input } satisfies SurfaceLayout;
        await command('set_surface_layout', { layout });
        capability = 'supported';
        return;
      } catch (failure) {
        if (!missing(failure)) throw failure;
        capability = 'missing';
      }
    }
    // Legacy path: no occlusion support, so floating UI cannot overlay native websites.
    await command('set_layout', { layout: input.services });
  }

  return {
    get capability() {
      return capability;
    },
    /** Latest request wins; resolves once no newer request is pending. */
    async request(collect: () => SurfaceInput) {
      const mine = ++latest;
      queue = queue
        .catch(() => {})
        .then(async () => {
          if (mine === latest) await send(collect());
        });
      let current: Promise<void>;
      do {
        current = queue;
        await current;
      } while (current !== queue);
    },
  };
}