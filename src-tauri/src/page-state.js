(() => {
  if (window.top !== window || !window.chrome?.webview) return;
  let dirty = false, pending = 0, last = '';
  // Sample the service edge rather than guessing a theme from its domain. Composite
  // translucent ancestors and report only data; the page cannot change shell settings.
  function background() {
    let node = document.elementFromPoint(2, Math.max(2, Math.floor(innerHeight / 2)));
    const layers = [];
    while (node) {
      const channels = getComputedStyle(node).backgroundColor.match(/[\d.]+/g)?.map(Number);
      if (channels?.length >= 3) {
        layers.push(channels);
        if ((channels[3] ?? 1) >= 1) break;
      }
      node = node.parentElement;
    }
    let rgb = matchMedia('(prefers-color-scheme: dark)').matches ? [24, 24, 27] : [255, 255, 255];
    for (const layer of layers.reverse()) {
      const alpha = layer[3] ?? 1;
      rgb = rgb.map((value, index) => Math.round(layer[index] * alpha + value * (1 - alpha)));
    }
    return rgb;
  }
  function report() {
    pending = 0;
    const elements = [...document.querySelectorAll('[class],[id]')].slice(0, 1000);
    const classes = [...new Set(elements.flatMap(element => [...element.classList]))].filter(value => value.length <= 128).slice(0, 300);
    const ids = [...new Set(elements.map(element => element.id).filter(Boolean))].filter(value => value.length <= 128).slice(0, 300);
    const media = [...document.querySelectorAll('audio,video')].some(element => !element.paused && !element.ended);
    const payload = JSON.stringify({ kind: 'page-state', dirty, media, classes, ids, background: background() });
    if (payload !== last) { last = payload; window.chrome.webview.postMessage(payload); }
  }
  function schedule() { if (!pending) pending = setTimeout(report, 300); }
  document.addEventListener('input', () => { dirty = true; schedule(); }, true);
  document.addEventListener('play', schedule, true);
  document.addEventListener('pause', schedule, true);
  document.addEventListener('ended', schedule, true);
  document.addEventListener('DOMContentLoaded', () => {
    new MutationObserver(schedule).observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'id', 'style', 'data-theme'] });
    schedule();
  }, { once: true });
  document.addEventListener('visibilitychange', schedule);
  window.addEventListener('resize', schedule);
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', schedule);
})();
