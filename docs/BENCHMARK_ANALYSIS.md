# Bobola Performance Benchmark Analysis

Date: November 6, 2026
Build: Windows x64 optimized benchmark harness, separate from the distributable executable
Machine: Windows 11 Enterprise, Intel Core i5-9400, 6 logical processors, 32 GiB RAM

## Executive Summary

The strongest measured result is process consolidation, not a verified RAM saving against Ferdium. With five identical local service pages, one Bobola workspace used a median **640 MiB private commit**. Five independent Bobola windows using one identical service each used **1,676 MiB**. That is approximately **1,036 MiB (1.01 GiB, 61.8%) less private commit** in this controlled comparison. Summed working set was 1,149 MiB versus 3,430 MiB, a 66.5% reduction; summed working set can double-count shared pages.

In the local ad-like network fixture, protection blocked a median **65 requests per roughly 9.5-second measured window**, and zero ad-like responses reached the fixture server. With protection off, a median 64 responses of 64 KiB each reached it, or **4 MiB transferred per window**. These are deliberately generated test payloads, not a real-site ad-removal or data-savings guarantee.

## Resource Results

Each case was run three times with reversed order on alternating runs. Pages were served from an owned loopback fixture. Each page rendered 48 simple DOM tiles and made one 64 KiB ad-like request and one 2-byte first-party request per second. The app and every descendant WebView2 process were sampled every two seconds for approximately ten seconds. Reported case values are the median of the three per-run medians.

| Workload                                          | Private commit | Summed working set | Processes | CPU (one core) | Fresh profile data |
| ------------------------------------------------- | -------------: | -----------------: | --------: | -------------: | -----------------: |
| Bobola shell, 0 services                          |        140 MiB |            323 MiB |         7 |           0.5% |            8.3 MiB |
| 1 Bobola workspace, 5 services, protection on     |        640 MiB |          1,149 MiB |        17 |          19.8% |           42.7 MiB |
| 1 Bobola workspace, 5 services, protection off    |        642 MiB |          1,152 MiB |        17 |          22.8% |           46.7 MiB |
| 5 Bobola instances, 1 service each, protection on |      1,676 MiB |          3,430 MiB |        65 |          41.4% |           48.9 MiB |

Consolidating the five pages into one Bobola workspace reduced private commit by 1,036 MiB (61.8%), summed working set by 2,281 MiB (66.5%), and process count by 48 (73.8%) compared with five separate Bobola instances. This isolates the app-consolidation effect on this machine. It does **not** show a 61.8% saving versus Ferdium or any other competitor. The three-run private-commit range was 638-645 MiB consolidated and 1,665-1,683 MiB across five instances.

Measured fixture CPU was a median 19.8% of one core for the consolidated workspace versus 41.4% for five instances, approximately 52% lower. This is the synthetic page's repeated local request workload on a short measurement window; it is not an idle, battery, or real-site CPU claim.

Protection-on and protection-off five-service runs were within normal run variation: private commit differed by about 1.6 MiB and summed working set by about 3.2 MiB. Do not advertise filtering as a RAM-saving feature based on these measurements.

The 0-to-5-service profile change was about 34.4 MiB across five new, empty WebView2 account profiles, approximately 6.9 MiB per service averaged in this fixture. Five separate Bobola instances measured 48.9 MiB combined profile data, compared with 42.7 MiB for the consolidated run. This is initial browser-profile metadata/cache overhead, not a forecast of long-term storage for real websites. Real site caches, cookies, offline data and user content can grow substantially and vary by service.

## Blocking And Throughput

Native request test, five services, three runs:

| Protection | Ad-like requests reaching local server | Ad-like response bytes | First-party fixture requests | Native blocked count |
| ---------- | -------------------------------------: | ---------------------: | ---------------------------: | -------------------: |
| On         |                                      0 |                      0 |                    median 64 |            median 65 |
| Off        |                              median 64 |           median 4 MiB |                    median 64 |                    0 |

First-party requests continued to reach the local server in both modes, which checks that this was selective blocking rather than a disconnected fixture. A prior runner revision counted the short startup period in the counter but counted transferred fixture bytes over the timed interval; the values above use the final timed-window-only runner and are internally consistent.

The separate optimized Rust-engine microbenchmark checks 100,000 requests per mode against the bundled three-rule starter list and generated exact-domain lists of 1,000 and 10,000 rules:

| Ruleset                             | Compile time | Enabled matching throughput | Mean enabled match time |      Disabled fast path |
| ----------------------------------- | -----------: | --------------------------: | ----------------------: | ----------------------: |
| Bundled starter, 3 rules            |      0.39 ms |          478,000 requests/s |                 2.09 us | 8.79 million requests/s |
| Synthetic domain list, 1,000 rules  |      0.84 ms |          474,000 requests/s |                 2.11 us | 8.63 million requests/s |
| Synthetic domain list, 10,000 rules |      6.83 ms |          454,000 requests/s |                 2.20 us | 8.66 million requests/s |

Each enabled mode blocked exactly 50,000 of 100,000 generated matching/nonmatching requests. Its purpose is matcher throughput and compilation time; it is not end-to-end browser latency, a real-list benchmark, or a comparison against AdGuard/uBlock Origin.

## Disk Footprint

- Current Bobola Windows executable: **16,393,728 bytes (15.63 MiB)**, SHA-256 `c3dcd46288b4b58347fb6076522932b9bd20e1f516d97bb4a9d6b29de00aa20a`.
- Ferdium executable in this environment: **225,584,128 bytes (215.13 MiB)**.
- The local Ferdium installation directory measured **532.76 MiB across 622 files**. This is an inventory of this installation, not a matched installer/package comparison.

The executable files differ by about 92.7% in size, but this is not a like-for-like installed-footprint claim: Bobola uses the system Microsoft WebView2 runtime, while Ferdium's Electron distribution bundles Chromium runtime files. Bobola's current exe is useful evidence for a compact download, subject to the WebView2 runtime prerequisite. The storage numbers above exclude users' profile data.

## Ad-Blocking Product Wording

Bobola currently integrates Brave's Rust `adblock` engine. It is **not an AdGuard integration** and should not be marketed as “AdGuard.” New profiles start with a small original list containing three network rules. Users can optionally download EasyList and EasyPrivacy. The built-in native request and cosmetic-filter behavior is covered by local native fixtures; real-world block rate, tracker reduction, page-load improvement, memory usage under popular lists, and comparison with AdGuard/uBlock Origin have not been measured here.

Safe current wording: “Optional built-in content filtering, powered by the Rust adblock engine, with EasyList and EasyPrivacy support.” Avoid a universal percentage, “blocks all ads,” or claims about real-site bandwidth until broad matched site tests exist.

## Baseline Limits And Next Benchmarks

The running Ferdium workload was not used as the memory baseline: active service count, page content, foreground state, cache state and protection settings were not matched, and its authenticated profile should not be altered for a benchmark. Ferdium 7.2.3 was also launched with a fresh profile under `E:\Temp` and a remote-debugging port; the launcher exited with code 0 before persisting profile files. Its stderr briefly announced a DevTools endpoint, but no persistent isolated app process remained to host fixture pages. An earlier Ferdium memory snapshot is also explicitly uncontrolled in `docs/REVERSE_ENGINEERING.md`.

Recommended next steps before publishing cross-product savings:

1. Use a fresh, isolated Ferdium profile and a verified matching set of 2 and 5 service pages; run both apps with the same content, viewport, session state, WebView/browser version where feasible, protection state, and warm/cold cache state.
2. Capture at least 30 minutes after a five-minute warm-up, with 5+ alternating runs. Report private commit, working set, CPU time, process count, startup time, page readiness, and p50/p95 values. Keep system background load and power mode fixed.
3. Test real, legally usable ad/test pages across news, video, social and shopping categories. Record request counts by resource type, blocked bytes using response content lengths, DOM cosmetic outcomes, page load and interaction timing, and false-positive/breakage checks. Compare the same filter subscriptions and their revisions.
4. Measure idle/hibernated resource use, wake latency, memory recovery after closing services, CPU and battery/energy per hour, and disk growth after 1 day/1 week of real profile use.

Until that matched Ferdium study is complete, market the directly observed advantages: consolidation into one workspace, the small executable, optional filtering, and per-account isolation. Treat the RAM savings figure as an internal prototype result for consolidation only.

## Real-page comparison checkpoint

Date: November 7, 2026

A disposable Ferdium 7.2.3 profile was populated with the same seven public HTTPS pages used by Bobola:

- WhatsApp: `https://web.whatsapp.com/`
- Telegram: `https://web.telegram.org/a/`
- YouTube: `https://www.youtube.com/`
- Reddit: `https://www.reddit.com/`
- Google Gemini: `https://gemini.google.com/app`
- Discord: `https://discord.com/app`
- YouTube Music: `https://music.youtube.com/`

The profile survived a full quit and relaunch with all seven enabled services and was copied into fresh per-run profiles. Bobola and Ferdium were run in alternating order. Bobola used EasyList and EasyPrivacy downloaded from their public list URLs for the protection-on case. Ferdium 7.2.3 has no equivalent bundled content-filtering engine, so its protection state is recorded as unavailable rather than treated as an equivalent “off” configuration.

The completed three-run resource set used 10-second process sampling after a 15-second post-readiness wait. The sampler recorded the root process only because Windows CIM and `Get-Process` enumeration hung under this machine’s process load. The values below therefore describe the main process working set, not the complete app plus WebView/Electron child tree:

| App and condition                     | Main-process working set, median | Seven service records |             Navigation/readiness checkpoint |
| ------------------------------------- | -------------------------------: | --------------------: | ------------------------------------------: |
| Bobola, protection on, 3 runs         |                     **39.7 MiB** |              7 loaded | 2.18 s median maximum navigation completion |
| Ferdium, no filtering, 3 runs         |                    **288.3 MiB** |   7 attached services |              17.19 s median probe readiness |
| Bobola, protection off, 1 control run |                     **31.6 MiB** |              7 loaded |        1.67 s maximum navigation completion |
| Ferdium, paired control run           |                    **313.2 MiB** |   7 attached services |                      4.96 s probe readiness |

These root-process values are useful diagnostics for this matched run, but they are not a total-RAM comparison. A publishable memory claim still requires a reliable descendant-process sampler that includes Bobola’s WebView2 processes and Ferdium’s Electron renderer, GPU and utility processes under the same sampling boundary.

Bobola’s real-page request instrumentation recorded the following protection results during the measured windows:

| Bobola condition                              | Total instrumented requests, median | Native blocked requests, median |
| --------------------------------------------- | ----------------------------------: | ------------------------------: |
| EasyList + EasyPrivacy, protection on, 3 runs |                               1,443 |                          **22** |
| Protection off, 1 control run                 |                               1,449 |                           **0** |

The control confirms that the filtering path changed request decisions on the same seven public pages. The result is a small site sample and does not establish a real-world block percentage, bandwidth saving, or breakage rate. Ferdium’s browser-side request capture did not share Bobola’s native interception boundary, and its page readiness varied by service, so cross-product request and byte totals are omitted.

Raw outputs:

- `<temporary-directory>/service-workspace-real-page-benchmark-2026-11-07T01-03-35-113Z/real-page-results.json` — three alternating protection-on runs.
- `<temporary-directory>/service-workspace-real-page-benchmark-2026-11-07T01-14-20-670Z/real-page-results.json` — one protection-off control and its paired Ferdium run.

The benchmark harness now supports this real-page workflow through `scripts/real-page-benchmark.mjs`, with fresh Ferdium profile copies, filter-list revision capture, bounded Windows process sampling, and owned-process cleanup. The harness build remains separate from the distributable executable.

## Reproduction

### November 7 sampler validation

The resource path now uses a standalone native ToolHelp/process-API sampler instead of CIM or `Get-Process`. The in-app resource panel additionally seeds ownership with WebView2 environment process inventories for both shell and service environments. Owned PIDs are deduplicated. The benchmark runners build the sampler and reject partial measurements, missing browser children, disappeared roots, or duplicate PIDs. CPU intervals match PID plus process creation time and omit work from processes that appear or exit between samples.

A debug-build validation run with two identical local fixture pages captured **14 owned processes**, including WebView2 browser children. The protection-off run completed with median private commit **439,054,336 bytes (418.7 MiB)** and summed working set **846,761,984 bytes (807.5 MiB)**. All sampled measurements were available. Its timed fixture interval recorded 22 first-party requests and 22 ad-like responses totaling 1,441,792 bytes; native blocked count was zero. This short, single-run debug fixture validates collection and accounting, not production performance or a cross-product saving.

Raw completed report: `output/resource-benchmark-2026-11-07T13-28-11-448Z/resource-results.json`.

An earlier combined validation command reached its terminal time limit after saving shell and protection-on case measurements under `output/resource-benchmark-2026-11-07T13-26-23-463Z/`; it did not finish an overall report. The completed protection-on case captured 14 processes, 22 blocked requests, zero ad-like server responses, and sustained first-party requests. Treat these as sampler/fixture validation only. The historical seven-page root-only results above have not been replaced by a new full-process competitor comparison.

Build the isolated release harness (do not distribute this harness build):

```powershell
node scripts/rust.mjs build -p service-workspace --release --features benchmark-harness
npm run benchmark:resources -- 10 3
npm run benchmark:protection
```

The resource runner resolves the executable from `CARGO_TARGET_DIR` when no path is supplied. Pass an explicit executable path first when using a different build directory.

Raw process snapshots, per-run values and environment metadata are in `output/resource-benchmark-2026-11-06T16-38-40-477Z/resource-results.json`. The separate 64 KiB network fixture run is in `output/resource-benchmark-2026-11-06T16-30-16-279Z/resource-results.json`. The optimized engine throughput result was produced by `npm run benchmark:protection` on the same machine on November 6, 2026.
