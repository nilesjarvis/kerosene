# Retained audit evidence — 2026-09-24

These artifacts support the [end-to-end audit](../../end-to-end-performance-audit.md).
They contain counts/timings and screenshots from credential-free test sessions.
No captured public API response bodies or user configuration are included here.

| Artifact | Meaning |
| --- | --- |
| [provenance.json](provenance.json) | HEAD, working-tree scope, source-manifest digest, toolchain and measurement environment. |
| [live-startup.json](live-startup.json) | Short live launch; 437 attempts, 253 HTTP 429s, no input beyond screenshot capture. |
| [interaction.json](interaction.json) | 629.1-second offline native interaction and fault-injection session. |
| [memory-baseline.json](memory-baseline.json) | Original manager, fixed 80-second faster-stream scenario. |
| [memory-comparison.json](memory-comparison.json) | Same scenario with only staged HL receiver ownership changed. |
| [baseline checks](memory-baseline-checks.json) / [comparison checks](memory-comparison-checks.json) | All ten coverage checks passed in both runs. |
| [receiver-experiment.diff](receiver-experiment.diff) | Exact staged-manager difference; no production source edit. |
| [receiver-memory.png](receiver-memory.png) | RSS over the controlled stream and cooldown. |
| [rate-limited chart](12-rate-limited-chart.png) | Live candle without verified history during the 429 sequence. |
| [unrecovered chart](16-history-not-recovered.png) | History remained unavailable after the fixture recovered. |
| [final delayed selection](18-slow-switch-final.png) | HYPE displayed correctly after delayed, superseded symbol requests. |

Raw JSONL, all screenshots, build/test/lint logs, staged source and captured public
metadata remain in the ignored `target/e2e/` directory. The test and lint logs are
`target/e2e-tests.log` and `target/e2e-clippy.log`. They are local working artifacts,
not release assets. Reproduction commands are in [the harness guide](../../../tests/e2e/README.md).

`cpu_max_pct` is retained verbatim from psutil and can spike on extremely short
sampling intervals around queued actions. Use the time-weighted phase means for
comparisons. Startup-to-window and input-to-photon latency were not measured.
The comparison controls ownership and workload; it is one pair of runs rather
than a statistically powered hardware benchmark.
