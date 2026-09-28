import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";

export async function checkCalculations(tools: Map<string, any>, snapshotPath: string) {
  const original = await readFile(snapshotPath, "utf8");
  const originalFetch = globalThis.fetch;
  const cases: unknown[] = [];
  const base = JSON.parse(original);
  const invoke = async (name: string, params: Record<string, unknown>) => {
    const result = await tools.get(name).execute("calculation-check", params);
    return { result, payload: JSON.parse(result.content[0].text) };
  };
  globalThis.fetch = (async () => assert.fail("snapshot calculations must not fetch")) as any;
  try {
    const fills = [
      { coin: "BTC", size: "2", price: "100", fee: "1", fee_token: "USDC", closed_pnl: "3", side: "B", time_ms: 10 },
      { coin: " btc ", size: "1", price: "110", fee: "-0.5", fee_token: "usdc", closed_pnl: "-2", side: "A", time_ms: 20 },
      { coin: "ETH", size: "3", price: "20", fee: "0.1", fee_token: "ETH", closed_pnl: "4", side: "B", time_ms: 20 },
      { coin: "BTC", size: "1", price: "100", fee: "1", closed_pnl: "0", side: "?", time_ms: 30 },
      { coin: "", size: "NaN", price: "", fee: null, closed_pnl: "NaN", side: "?", time_ms: 0 },
      { coin: "LATE", size: 0, price: 0, fee: 0, closed_pnl: 0, side: "A", time_ms: null },
    ];
    const funding = [
      { coin: "BTC", usdc: "2", time_ms: 10 },
      { coin: " btc ", usdc: "-3", time_ms: 20 },
      { coin: "ETH", usdc: "0", time_ms: 20 },
      { coin: "BTC", usdc: "NaN", time_ms: 30 },
      { coin: "", usdc: "bad", time_ms: 0 },
      { coin: "LATE", usdc: "1.5", time_ms: null },
    ];
    const sourceCoverage = {
      fills: { truncated: true, endpoint_fetch_complete: false, source: "fill fixture" },
      funding: { truncated: false, endpoint_fetch_complete: true, source: "funding fixture" },
    };
    for (const [label, activity] of [
      ["mixed", { fills, funding, as_of_ms: 6_000, coverage: sourceCoverage }],
      ["reversed", { fills: [...fills].reverse(), funding: [...funding].reverse(), as_of_ms: 6_000, coverage: sourceCoverage }],
      ["empty", { fills: [], funding: [], as_of_ms: 6_000, coverage: sourceCoverage }],
      ["nonarray", { fills: {}, funding: null }],
      ["missing", undefined],
    ] as const) {
      const snapshot = structuredClone(base);
      snapshot._tool_data.activity = activity;
      await writeFile(snapshotPath, JSON.stringify(snapshot));
      for (const kind of ["fills", "funding"] as const) {
        const operation = kind === "fills" ? "fill_aggregation" : "funding_aggregation";
        for (const filter of [{}, { symbol: " btc " }, { start_ms: 10, end_ms: 20 },
          { symbol: "eth", start_ms: 20, end_ms: 20 }, { start_ms: 100 }, { start_ms: 21, end_ms: 9 }]) {
          const calculated = await invoke("kerosene_calculate", { operation, ...filter });
          const queried = await invoke("kerosene_activity", { kind, mode: "aggregate", ...filter });
          const aggregate = calculated.payload.result.aggregate;
          assert.deepEqual(aggregate, queried.payload.aggregate);
          assert.deepEqual(calculated.result.details, { operation });
          assert.equal(calculated.payload.operation, operation);
          assert.equal(calculated.payload.result.coverage.matched_rows, queried.payload.coverage.matched_rows);
          assert.deepEqual(calculated.payload.result.coverage.source, queried.payload.coverage.source);
          // These tools intentionally report different observation scopes.
          assert.equal(calculated.payload.quality.observed_at_ms, 9_000);
          assert.equal(queried.payload.quality.observed_at_ms, activity?.as_of_ms ?? null);
          if (label === "mixed" && Object.keys(filter).length === 0) {
            assert.equal(aggregate.validation.included_rows, 4);
            assert.equal(aggregate.validation.excluded_rows, 2);
            assert.deepEqual(calculated.payload.quality.warnings,
              ["2 malformed row(s) were excluded instead of treated as zero."]);
            if (kind === "fills") {
              assert.deepEqual(aggregate.by_coin.map((row: any) => row.coin), ["BTC", "ETH", "LATE"]);
              assert.equal(aggregate.by_coin[0].buy_notional, 200);
              assert.equal(aggregate.by_coin[0].sell_notional, 110);
              assert.equal(aggregate.by_coin[0].closed_pnl, 1);
              assert.deepEqual(aggregate.by_coin[0].fees_by_token, { USDC: 0.5 });
            } else {
              assert.deepEqual(aggregate.by_coin.map((row: any) => row.coin), ["BTC", "LATE", "ETH"]);
              assert.deepEqual(aggregate.total, {
                received_usdc: 3.5, paid_usdc: 3, net_usdc: 0.5, absolute_usdc: 6.5,
              });
            }
          }
          cases.push({ label, kind, filter, calculated: calculated.result, queried: queried.result });
        }
      }
    }

    const marketRows = [
      { symbol: "BTC", canonical_symbol: "BTC", display_symbol: "BTC", market_type: "perp", mid: 100 },
      { symbol: "ETH", canonical_symbol: "ETH", display_symbol: "ETH", market_type: "perp", mid: 50 },
      { symbol: "@1", canonical_symbol: "X", display_symbol: "X", market_type: "spot", mid: 10 },
      { symbol: "XPERP", canonical_symbol: "X", display_symbol: "X", market_type: "perp", mid: 20 },
    ];
    for (const [label, balances, positions] of [
      ["empty", [], []],
      ["nonarray", null, {}],
      ["combined", [{ coin: "USDC", total: "100" }, { coin: " btc ", total: "2" }, { coin: "ETH", total: "-1" }],
        [{ coin: "BTC", size: "-1" }, { coin: "ETH", size: "3", position_value: "999" }]],
      ["duplicates", [{ coin: "BTC", total: 1 }, { coin: "btc", total: -1 }, { coin: "USDC", total: 2 }],
        [{ coin: "BTC", size: 2 }, { coin: "BTC", size: -2 }]],
      ["missing prices", [{ coin: "MISSING", total: 3 }, { coin: "MISSING", total: 2 }],
        [{ coin: "MISSING", size: 1 }, { coin: "OTHER", size: 0 }]],
      ["fallback signs", [], [{ coin: "SHORT", size: -2, position_value: -300 },
        { coin: "LONG", size: 2, position_value: -300 }, { coin: "ZERO", size: 0, position_value: -10 }]],
      ["invalid", [{ coin: "", total: "bad" }, { coin: "BAD", total: "NaN" }, { coin: "BOOL", total: true }],
        [{ coin: "", size: "bad" }, { coin: "BAD", size: "Infinity" }, { coin: "BOOL", size: false }]],
      ["zero balance", [{ coin: "UNKNOWN", total: 0 }], []],
      ["canonical preference", [{ coin: "X", total: 2 }], [{ coin: "X", size: -1 }]],
      ["sum order", [{ coin: "USDC", total: 1e16 }, { coin: "USDC", total: 1 }, { coin: "USDC", total: -1e16 }], []],
    ] as const) {
      const snapshot = structuredClone(base);
      snapshot.account.spot.balances = balances;
      snapshot.account.positions = positions;
      snapshot._tool_data.markets.rows = marketRows;
      await writeFile(snapshotPath, JSON.stringify(snapshot));
      const exposure = await invoke("kerosene_calculate", { operation: "exposure" });
      const value = exposure.payload.result;
      if (label === "combined") {
        assert.deepEqual(value.by_asset.map((row: any) => [row.coin, row.net_value_usd]),
          [["USDC", 100], ["BTC", 100], ["ETH", 100]]);
        assert.equal(value.gross_observable_value_usd, 300);
        assert.equal(value.net_observable_value_usd, 300);
        assert.deepEqual(value.by_asset[1].valuation_notes, ["current_mid", "current_mid"]);
      } else if (label === "missing prices") {
        assert.deepEqual(value.missing_price_symbols, ["MISSING", "OTHER"]);
        assert.equal(value.validation.fully_valued, false);
        assert.equal(value.validation.excluded_rows, 0);
      } else if (label === "fallback signs") {
        assert.deepEqual(value.by_asset.map((row: any) => [row.coin, row.net_value_usd]),
          [["SHORT", -300], ["LONG", 300], ["ZERO", 10]]);
      } else if (label === "invalid") {
        assert.deepEqual(value.validation.exclusion_reasons,
          { missing_coin: 1, invalid_balance: 2, missing_position_coin: 1, invalid_position_size: 2 });
        assert.equal(value.validation.excluded_rows, 6);
      } else if (label === "zero balance") {
        assert.deepEqual(value.by_asset[0].valuation_notes, ["zero_balance"]);
        assert.equal(value.validation.fully_valued, true);
      } else if (label === "canonical preference") {
        assert.equal(value.by_asset[0].spot_value_usd, 40);
        assert.equal(value.by_asset[0].perp_value_usd, -20);
      } else if (label === "sum order") {
        assert.equal(value.net_observable_value_usd, 0);
      }
      cases.push({ label, params: { operation: "exposure" }, result: exposure.result });
      for (const options of [{}, { shock_pct: -10 }, { shock_pct: 0 }, { shock_pct: 5, include_stablecoins_in_shock: true }]) {
        const params = { operation: "stress", ...options };
        const stress = await invoke("kerosene_calculate", params);
        assert.deepEqual(stress.payload.result.exposure_coverage, value);
        if (label === "combined") {
          assert.equal(stress.payload.result.delta_equity_usd,
            options.shock_pct === -10 ? -20 : options.shock_pct === 0 ? 0 : options.include_stablecoins_in_shock ? 15 : 10);
        }
        cases.push({ label, params, result: stress.result });
      }
    }
    if (process.env.KEROSENE_AGENT_CALCULATION_CHECK_OUTPUT) {
      await writeFile(process.env.KEROSENE_AGENT_CALCULATION_CHECK_OUTPUT, JSON.stringify(cases, null, 2));
    }
  } finally {
    globalThis.fetch = originalFetch;
    await writeFile(snapshotPath, original);
  }
}
