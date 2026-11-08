import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';

const revision = 'bc708f6aad02fc43168e976d1d65678eaf96f21b';
const ids = [
  'whatsapp',
  'telegram',
  'discord',
  'slack',
  'msteams',
  'messenger',
  'hangoutschat',
  'gmail',
  'office365-owa',
  'proton-mail',
  'yahoo-mail',
  'fastmail',
  'google-calendar',
  'google-drive',
  'google-keep',
  'notion',
  'trello',
  'todoist',
  'asana',
  'github',
  'gitlab',
  'linkedin',
  'instagram',
  'reddit',
  'twitter',
  'youtube',
  'youtubemusic',
  'chatgpt',
  'claude-ai',
  'google-gemini',
];
const live = new Set(ids.slice(0, 12));
const hosts = {
  whatsapp: ['web.whatsapp.com'],
  telegram: ['web.telegram.org'],
  discord: ['discord.com'],
  slack: ['slack.com'],
  msteams: ['teams.microsoft.com', 'teams.live.com', 'teams.cloud.microsoft'],
  messenger: ['messenger.com'],
  hangoutschat: ['chat.google.com'],
  gmail: ['mail.google.com'],
  'office365-owa': ['outlook.office.com', 'outlook.office365.com', 'outlook.live.com'],
  'proton-mail': ['mail.proton.me', 'mail.protonmail.com'],
  'yahoo-mail': ['mail.yahoo.com'],
  fastmail: ['app.fastmail.com'],
  'google-calendar': ['calendar.google.com'],
  'google-drive': ['drive.google.com'],
  'google-keep': ['keep.google.com'],
  notion: ['notion.so'],
  trello: ['trello.com'],
  todoist: ['app.todoist.com', 'todoist.com'],
  asana: ['app.asana.com'],
  github: ['github.com'],
  gitlab: ['gitlab.com'],
  linkedin: ['linkedin.com'],
  instagram: ['instagram.com'],
  reddit: ['reddit.com'],
  twitter: ['x.com', 'twitter.com'],
  youtube: ['www.youtube.com', 'youtube.com'],
  youtubemusic: ['music.youtube.com'],
  chatgpt: ['chatgpt.com', 'chat.openai.com'],
  'claude-ai': ['claude.ai'],
  'google-gemini': ['gemini.google.com'],
};
const catalog = JSON.parse(await fs.readFile(new URL('../src/lib/catalog.json', import.meta.url)));
const output = new URL('../public/service-icons/', import.meta.url);
await fs.mkdir(output, { recursive: true });
const entries = [];
for (const id of ids) {
  const entry = catalog.find((entry) => entry.id === id);
  if (!entry) throw new Error(`Missing catalog entry: ${id}`);
  const source = `https://raw.githubusercontent.com/ferdium/ferdium-recipes/${revision}/recipes/${id}/icon.svg`;
  const response = await fetch(source, { signal: AbortSignal.timeout(30000) });
  if (!response.ok) throw new Error(`${id}: ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  const svg = bytes.toString('utf8');
  if (
    bytes.length > 2 * 1024 * 1024 ||
    !/<svg[\s>]/i.test(svg) ||
    /<script|<foreignObject|\bon\w+\s*=|(?:href|src)\s*=\s*["']\s*(?:https?:|javascript:|\/\/)/i.test(
      svg
    )
  )
    throw new Error(`Unsafe or unexpected artwork: ${id}`);
  await fs.writeFile(new URL(`${id}.svg`, output), bytes);
  entries.push({
    ...entry,
    icon: `/service-icons/${id}.svg`,
    hosts: hosts[id],
    keepLive: live.has(id),
    source,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  });
}
await fs.writeFile(
  new URL('../src/lib/popular-services.json', import.meta.url),
  JSON.stringify(entries, null, 2) + '\n'
);
await fs.writeFile(
  new URL('../SERVICE_ARTWORK_PROVENANCE.json', import.meta.url),
  JSON.stringify(
    {
      source: 'https://github.com/ferdium/ferdium-recipes',
      revision,
      selection: 'Curated popular services; not a measured usage ranking.',
      content:
        'Unmodified SVG artwork from the pinned recipes. Recipe scripts are not executed or bundled. Brand trademarks remain with their owners.',
      entries: entries.map(({ id, license, source, sha256 }) => ({ id, license, source, sha256 })),
    },
    null,
    2
  ) + '\n'
);
console.log(`Imported ${entries.length} verified local service icons.`);
