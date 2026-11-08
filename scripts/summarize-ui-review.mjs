import { readFile } from 'node:fs/promises';
const percentile = (values, p) => [...values].sort((a,b) => a-b)[Math.ceil(values.length*p)-1];
for (const label of ['before', 'after']) {
  const report = JSON.parse(await readFile(`output/ui-${label}/measurements.json`, 'utf8'));
  console.log(label, report.results.map(result => ({ scale: result.scale,
    readyMs: Math.round(result.startupToAutomationReadyMs),
    switchMedianMs: +percentile(result.switches, .5).toFixed(2),
    switchP95Ms: +percentile(result.switches, .95).toFixed(2),
    frameMedianMs: +percentile(result.frames, .5).toFixed(2),
    frameP95Ms: +percentile(result.frames, .95).toFixed(2),
  })));
}
