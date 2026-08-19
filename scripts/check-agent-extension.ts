import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import keroseneExtension from "../assets/agent/kerosene.ts";

const tools = new Map<string, any>();
const hooks = new Map<string, any>();
keroseneExtension({
  registerTool(tool: any) {
    tools.set(tool.name, tool);
  },
  on(name: string, hook: any) {
    hooks.set(name, hook);
  },
} as any);

async function execute(name: string, params: Record<string, unknown>) {
  const result = await tools.get(name).execute("test-call", params);
  return JSON.parse(result.content[0].text);
}

const workspace = await mkdtemp(join(tmpdir(), "kerosene-agent-extension-test-"));
try {
  const snapshotPath = join(workspace, "snapshot.json");
  process.env.KEROSENE_AGENT_SNAPSHOT = snapshotPath;
  await writeFile(snapshotPath, JSON.stringify({
    schema_version: 3,
    generated_at_ms: 10_000,
    data_policy: { access: "read_only" },
    account: {
      available: true,
      provenance: { source: "fixture", observed_at_ms: 9_000, as_of_ms: 9_000 },
      positions: [],
      spot: { balances: [] },
    },
    markets: { markets: [], coverage: { returned_count: 0, total_count: 0, truncated: false } },
    journal: { available: true, data_state: "ready" },
    _tool_data: {
      markets: {
        as_of_ms: 9_500,
        rows: [{ symbol: "BTC", canonical_symbol: "BTC", display_symbol: "BTC", market_type: "perp", mid: 100 }],
        coverage: { returned_count: 1, total_count: 1, truncated: false },
      },
      activity: {
        as_of_ms: 9_000,
        fills: [
          { coin: "BTC", size: "1", price: "100", fee: "1", closed_pnl: "4", side: "A", time_ms: 1 },
          { coin: "BTC", size: "2", price: "100", fee: "1", closed_pnl: "5", side: "?", time_ms: 2 },
        ],
        funding: [],
        coverage: { fills: { returned_count: 2, total_count: 2, truncated: false } },
      },
      journal: {
        available: true,
        as_of_ms: 9_000,
        data_state: "ready",
        coverage: { returned_count: 2, total_count: 2, truncated: false, endpoint_fetch_complete: true },
        trades: [
          { status: "CLOSED", start_time_ms: 1, net_realized_pnl_usd: 5, gross_realized_pnl_usd: 6, fees_usd: 1, basis_complete: true },
          { status: "CLOSED", start_time_ms: 2, net_realized_pnl_usd: null, gross_realized_pnl_usd: null, fees_usd: null, basis_complete: true },
        ],
      },
      risk: { available: false, as_of_ms: null },
    },
  }));

  const fills = await execute("kerosene_activity", { kind: "fills", mode: "aggregate" });
  assert.equal(fills.aggregate.validation.included_rows, 1);
  assert.equal(fills.aggregate.validation.excluded_rows, 1);
  assert.equal(fills.aggregate.validation.exclusion_reasons.unknown_side, 1);
  assert.equal(fills.aggregate.by_coin[0].sell_size, 1);
  assert.match(fills.quality.warnings[0], /excluded instead of treated as zero/);

  const journal = await execute("kerosene_journal", { operation: "summary" });
  assert.equal(journal.summary.overall.net_realized_pnl_usd, 5);
  assert.equal(journal.summary.overall.flats, 0);
  assert.equal(journal.summary.overall.win_rate_sample_count, 1);
  assert.equal(journal.summary.overall.metric_coverage.net_pnl.missing_rows, 1);

  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => ({
    ok: true,
    json: async () => [
      { t: 1, T: 2, o: "100", h: "110", l: "95", c: "105", v: "10" },
      { t: 3, T: 4, o: "105", h: "115", l: "100", c: "110", v: "12" },
      { t: 5, T: 6, o: "110", h: "112", l: "90", c: "95", v: "14" },
    ],
  })) as any;
  try {
    const market = await execute("kerosene_calculate", {
      operation: "market_statistics",
      symbol: "BTC",
      interval: "1h",
      start_ms: 1,
      end_ms: 10_000,
      limit: 3,
    });
    assert.equal(market.result.statistics.sample_count, 3);
    assert.equal(market.result.statistics.last_close, 95);
    assert.ok(market.result.statistics.maximum_close_drawdown_pct > 0);
    assert.equal(market.quality.source, "hyperliquid_candleSnapshot_computed_by_kerosene");
  } finally {
    globalThis.fetch = originalFetch;
  }

  const prompt = await hooks.get("before_agent_start")({ systemPrompt: "base" });
  assert.match(prompt.systemPrompt, /Ground every material claim in evidence retrieved during the current turn/);
  assert.match(prompt.systemPrompt, /If sources conflict, expose the conflict/);
} finally {
  await rm(workspace, { recursive: true });
}
