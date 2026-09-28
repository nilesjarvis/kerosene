import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";

export async function checkSessions(tools: Map<string, any>) {
  const originalFetch = globalThis.fetch;
  const originalNow = Date.now;
  const cases: unknown[] = [];
  const requests: any[] = [];
  // Fixed synthetic returns across the separate US and UK clock changes.
  const transitionSummaries: Record<string, Array<[string, number, number]>> = {
    "2026-03-09": [["Asia", 7, 0.7932711184197875], ["London", 7, -2.179033724000497],
      ["New York", 6, 5.788264250151438], ["Overnight", 6, 5.418769438440535]],
    "2026-03-30": [["Asia", 7, -0.11912437412837663], ["London", 7, -14.380251090952315],
      ["New York", 7, -1.4687748834617107], ["Overnight", 5, 6.815671590252228]],
    "2026-10-26": [["Asia", 7, -0.2393206961877186], ["London", 7, -13.871475647072506],
      ["New York", 7, -1.4687748834617107], ["Overnight", 5, 6.815671590252228]],
    "2026-11-02": [["Asia", 7, 0.7932711184197875], ["London", 7, -13.905739259987454],
      ["New York", 7, -2.32175294225225], ["Overnight", 5, 6.482663460170928]],
  };
  globalThis.fetch = (async (url: string, options: any) => {
    assert.equal(url, "https://api.hyperliquid.xyz/info");
    const body = JSON.parse(options.body);
    assert.equal(body.type, "candleSnapshot");
    assert.equal(body.req.coin, "BTC");
    assert.equal(options.method, "POST");
    requests.push(body);
    const interval = body.req.interval === "1d" ? 86_400_000 : 30 * 60_000;
    assert.ok(["1d", "30m"].includes(body.req.interval));
    const candles = [];
    for (let time = body.req.startTime, index = 0; time < body.req.endTime; time += interval, index += 1) {
      const open = index % 31 === 0 ? 0 : 100 + index % 37;
      const close = open + index % 11 - 5;
      candles.push({ t: time, T: time + interval - 1, o: open, h: Math.max(open, close),
        l: Math.min(open, close), c: close, v: index });
    }
    return { ok: true, json: async () => candles };
  }) as any;
  try {
    for (const date of [
      "2024-02-29", "2026-01-01", "2026-03-09", "2026-03-30",
      "2026-07-01", "2026-10-26", "2026-11-02", "2026-12-31",
    ]) {
      const now = Date.parse(`${date}T23:00:00Z`);
      Date.now = () => now;
      for (const lookbackDays of [7, 90]) {
        requests.length = 0;
        const result = await tools.get("kerosene_sessions").execute("sessions", {
          symbol: "BTC", lookback_days: lookbackDays,
        });
        const payload = JSON.parse(result.content[0].text);
        assert.equal(payload.available, true);
        assert.equal(payload.requested_start_ms, now - lookbackDays * 86_400_000);
        assert.equal(payload.requested_end_ms, now);
        assert.equal(payload.retrieved_at_ms, now);
        assert.equal(payload.daily_sample_count, lookbackDays);
        assert.equal(payload.intraday_sample_count, lookbackDays * 48);
        assert.deepEqual(payload.market_session_summaries.map((row: any) => row.label),
          ["Asia", "London", "New York", "Overnight"]);
        if (lookbackDays === 7 && transitionSummaries[date]) {
          assert.deepEqual(payload.market_session_summaries.map((row: any) =>
            [row.label, row.sample_count, row.average_return_pct]), transitionSummaries[date]);
        }
        assert.deepEqual(requests.map((request) => request.req.interval), ["1d", "30m"]);
        cases.push({ date, lookback_days: lookbackDays, requests: [...requests], result });
      }
    }
    if (process.env.KEROSENE_AGENT_SESSION_CHECK_OUTPUT) {
      await writeFile(process.env.KEROSENE_AGENT_SESSION_CHECK_OUTPUT, JSON.stringify(cases, null, 2));
    }
  } finally {
    globalThis.fetch = originalFetch;
    Date.now = originalNow;
  }
}
