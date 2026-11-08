import popular from './popular-services.json';

/** Only trusted local assets are returned. Host matching respects DNS label boundaries. */
export function getServiceIcon(name: string, url = '', catalogId?: string | null): string | null {
  const exact = popular.find((entry) => entry.id === catalogId);
  if (exact) return exact.icon;
  try {
    const host = new URL(url).hostname.toLowerCase();
    const matches = popular.flatMap((entry) =>
      entry.hosts
        .filter((domain) => host === domain || host.endsWith(`.${domain}`))
        .map((domain) => ({ entry, specificity: domain.length }))
    );
    matches.sort((a, b) => b.specificity - a.specificity);
    if (matches.length) return matches[0].entry.icon;
    // A custom URL is authoritative; its account label must not impersonate another service.
    if (url) return null;
  } catch {
    if (url) return null;
  }
  return (
    popular.find((entry) => entry.name.toLowerCase() === name.trim().toLowerCase())?.icon ?? null
  );
}
