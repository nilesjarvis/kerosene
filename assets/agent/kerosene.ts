import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { readFile } from "node:fs/promises";

const PUBLIC_SECTION_NAMES = [
  "overview",
  "account",
  "portfolio",
  "markets",
  "positioning",
  "sessions",
] as const;
const PUBLIC_SECTION_INPUTS = [...PUBLIC_SECTION_NAMES, "all"] as const;
const HYPERLIQUID_INFO_URL = "https://api.hyperliquid.xyz/info";
const HYPERDASH_API_URL = "https://api.hyperdash.com/graphql";
const MAX_MARKET_SYMBOLS = 20;
const MAX_ACTIVITY_ROWS = 200;
const MAX_POSITIONING_SYMBOLS = 3;
const MAX_CANDLES = 500;
const MAX_CANDLE_LOOKBACK_MS = 90 * 24 * 60 * 60_000;
const CANDLE_INTERVAL_MS: Record<string, number> = {
  "1m": 60_000,
  "3m": 3 * 60_000,
  "5m": 5 * 60_000,
  "15m": 15 * 60_000,
  "30m": 30 * 60_000,
  "1h": 60 * 60_000,
  "2h": 2 * 60 * 60_000,
  "4h": 4 * 60 * 60_000,
  "8h": 8 * 60 * 60_000,
  "12h": 12 * 60 * 60_000,
  "1d": 24 * 60 * 60_000,
};
const STABLECOINS = new Set(["USDC", "USDT", "USDT0", "USDE", "USDH"]);

type JsonObject = Record<string, any>;

async function readSnapshot(): Promise<JsonObject> {
  const snapshotPath = process.env.KEROSENE_AGENT_SNAPSHOT;
  if (!snapshotPath) throw new Error("KEROSENE_AGENT_SNAPSHOT is not configured");
  const snapshot = JSON.parse(await readFile(snapshotPath, "utf8"));
  if (!snapshot || typeof snapshot !== "object") throw new Error("Kerosene snapshot is invalid");
  return snapshot;
}

function toolPayload(payload: unknown, details: JsonObject = {}) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(payload) }],
    details,
  };
}

function publicSnapshot(snapshot: JsonObject) {
  return {
    schema_version: snapshot.schema_version,
    generated_at_ms: snapshot.generated_at_ms,
    data_policy: snapshot.data_policy,
    ...Object.fromEntries(PUBLIC_SECTION_NAMES.map((name) => [name, snapshot[name]])),
  };
}

function finiteNumber(value: unknown): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function numericOrZero(value: unknown): number {
  return finiteNumber(value) ?? 0;
}

function activityRows(snapshot: JsonObject, kind: "fills" | "funding"): JsonObject[] {
  const rows = snapshot?._tool_data?.activity?.[kind];
  return Array.isArray(rows) ? rows : [];
}

function marketRows(snapshot: JsonObject): JsonObject[] {
  const rows = snapshot?._tool_data?.markets?.rows;
  if (Array.isArray(rows)) return rows;
  return Array.isArray(snapshot?.markets?.markets) ? snapshot.markets.markets : [];
}

function normalizedSymbol(value: unknown): string {
  return typeof value === "string" ? value.trim().toUpperCase() : "";
}

function resolveMarkets(snapshot: JsonObject, query: string): JsonObject[] {
  const needle = normalizedSymbol(query);
  if (!needle) return [];
  const rows = marketRows(snapshot);
  const exactRaw = rows.filter((row) => normalizedSymbol(row.symbol) === needle);
  if (exactRaw.length) return exactRaw;
  const canonical = rows.filter((row) => normalizedSymbol(row.canonical_symbol) === needle);
  if (canonical.length) return canonical;
  return rows.filter((row) => normalizedSymbol(row.display_symbol) === needle);
}

function preferredMarket(snapshot: JsonObject, query: string): JsonObject | null {
  const matches = resolveMarkets(snapshot, query);
  return (
    matches.find((row) => normalizedSymbol(row.symbol) === normalizedSymbol(query)) ??
    matches.find((row) => row.market_type === "perp") ??
    matches[0] ??
    null
  );
}

function coverage(returnedCount: number, totalCount: number, extra: JsonObject = {}) {
  return {
    returned_count: returnedCount,
    total_count: totalCount,
    truncated: returnedCount < totalCount,
    ...extra,
  };
}

function filterActivity(rows: JsonObject[], params: JsonObject): JsonObject[] {
  const symbol = normalizedSymbol(params.symbol);
  const startMs = finiteNumber(params.start_ms);
  const endMs = finiteNumber(params.end_ms);
  return rows.filter((row) => {
    if (symbol && normalizedSymbol(row.coin) !== symbol) return false;
    const time = finiteNumber(row.time_ms);
    if (startMs !== null && (time === null || time < startMs)) return false;
    if (endMs !== null && (time === null || time > endMs)) return false;
    return true;
  });
}

function aggregateFills(rows: JsonObject[]) {
  const groups = new Map<string, JsonObject>();
  for (const row of rows) {
    const coin = normalizedSymbol(row.coin) || "UNKNOWN";
    const group = groups.get(coin) ?? {
      coin,
      row_count: 0,
      buy_size: 0,
      sell_size: 0,
      buy_notional: 0,
      sell_notional: 0,
      closed_pnl: 0,
      fees_by_token: {},
      first_time_ms: null,
      last_time_ms: null,
    };
    const size = numericOrZero(row.size);
    const price = numericOrZero(row.price);
    const fee = numericOrZero(row.fee);
    const time = finiteNumber(row.time_ms);
    const isBuy = normalizedSymbol(row.side) === "B";
    group.row_count += 1;
    group.closed_pnl += numericOrZero(row.closed_pnl);
    if (isBuy) {
      group.buy_size += size;
      group.buy_notional += size * price;
    } else {
      group.sell_size += size;
      group.sell_notional += size * price;
    }
    const feeToken = normalizedSymbol(row.fee_token) || "UNKNOWN";
    group.fees_by_token[feeToken] = (group.fees_by_token[feeToken] ?? 0) + fee;
    if (time !== null) {
      group.first_time_ms = group.first_time_ms === null ? time : Math.min(group.first_time_ms, time);
      group.last_time_ms = group.last_time_ms === null ? time : Math.max(group.last_time_ms, time);
    }
    groups.set(coin, group);
  }
  return [...groups.values()].sort((left, right) => right.row_count - left.row_count);
}

function aggregateFunding(rows: JsonObject[]) {
  const groups = new Map<string, JsonObject>();
  for (const row of rows) {
    const coin = normalizedSymbol(row.coin) || "UNKNOWN";
    const group = groups.get(coin) ?? {
      coin,
      row_count: 0,
      received_usdc: 0,
      paid_usdc: 0,
      net_usdc: 0,
      absolute_usdc: 0,
      first_time_ms: null,
      last_time_ms: null,
    };
    const cashFlow = numericOrZero(row.usdc);
    const time = finiteNumber(row.time_ms);
    group.row_count += 1;
    group.net_usdc += cashFlow;
    group.absolute_usdc += Math.abs(cashFlow);
    if (cashFlow >= 0) group.received_usdc += cashFlow;
    else group.paid_usdc += Math.abs(cashFlow);
    if (time !== null) {
      group.first_time_ms = group.first_time_ms === null ? time : Math.min(group.first_time_ms, time);
      group.last_time_ms = group.last_time_ms === null ? time : Math.max(group.last_time_ms, time);
    }
    groups.set(coin, group);
  }
  const byCoin = [...groups.values()].sort((left, right) => right.absolute_usdc - left.absolute_usdc);
  return {
    sign_convention: "usdc < 0 means paid; usdc > 0 means received",
    by_coin: byCoin,
    total: {
      received_usdc: byCoin.reduce((sum, row) => sum + row.received_usdc, 0),
      paid_usdc: byCoin.reduce((sum, row) => sum + row.paid_usdc, 0),
      net_usdc: byCoin.reduce((sum, row) => sum + row.net_usdc, 0),
      absolute_usdc: byCoin.reduce((sum, row) => sum + row.absolute_usdc, 0),
    },
  };
}

function valueSpotBalance(snapshot: JsonObject, balance: JsonObject) {
  const coin = normalizedSymbol(balance.coin);
  const units = numericOrZero(balance.total);
  if (units === 0) return { coin, units, value_usd: 0, price: null, valuation_method: "zero_balance" };
  if (STABLECOINS.has(coin)) {
    return {
      coin,
      units,
      value_usd: units,
      price: 1,
      valuation_method: "stablecoin_par_assumption",
    };
  }
  const market = preferredMarket(snapshot, coin);
  const price = finiteNumber(market?.mid);
  return {
    coin,
    units,
    value_usd: price === null ? null : units * price,
    price,
    market_symbol: market?.symbol ?? null,
    valuation_method: price === null ? "missing_mid" : "current_mid",
  };
}

function calculateExposure(snapshot: JsonObject) {
  const account = snapshot.account ?? {};
  const positions = Array.isArray(account.positions) ? account.positions : [];
  const balances = Array.isArray(account.spot?.balances) ? account.spot.balances : [];
  const byAsset = new Map<string, JsonObject>();
  const missingPrices: string[] = [];

  for (const balance of balances) {
    const valued = valueSpotBalance(snapshot, balance);
    if (valued.value_usd === null && valued.units !== 0) missingPrices.push(valued.coin);
    const group = byAsset.get(valued.coin) ?? {
      coin: valued.coin,
      spot_units: 0,
      spot_value_usd: 0,
      perp_size: 0,
      perp_value_usd: 0,
      net_value_usd: 0,
      valuation_notes: [],
    };
    group.spot_units += valued.units;
    if (valued.value_usd !== null) {
      group.spot_value_usd += valued.value_usd;
      group.net_value_usd += valued.value_usd;
    }
    group.valuation_notes.push(valued.valuation_method);
    byAsset.set(valued.coin, group);
  }

  for (const position of positions) {
    const coin = normalizedSymbol(position.coin);
    const size = numericOrZero(position.size);
    const market = preferredMarket(snapshot, coin);
    const mid = finiteNumber(market?.mid);
    const reportedValue = finiteNumber(position.position_value);
    const signedValue = mid === null
      ? reportedValue === null ? null : Math.sign(size || 1) * Math.abs(reportedValue)
      : size * mid;
    if (signedValue === null) missingPrices.push(coin);
    const group = byAsset.get(coin) ?? {
      coin,
      spot_units: 0,
      spot_value_usd: 0,
      perp_size: 0,
      perp_value_usd: 0,
      net_value_usd: 0,
      valuation_notes: [],
    };
    group.perp_size += size;
    if (signedValue !== null) {
      group.perp_value_usd += signedValue;
      group.net_value_usd += signedValue;
    }
    group.valuation_notes.push(mid === null ? "reported_position_value" : "current_mid");
    byAsset.set(coin, group);
  }

  const rows = [...byAsset.values()].sort(
    (left, right) => Math.abs(right.net_value_usd) - Math.abs(left.net_value_usd),
  );
  const gross = rows.reduce((sum, row) => sum + Math.abs(row.net_value_usd), 0);
  const net = rows.reduce((sum, row) => sum + row.net_value_usd, 0);
  for (const row of rows) row.gross_share_pct = gross > 0 ? Math.abs(row.net_value_usd) / gross * 100 : 0;
  return {
    as_of_ms: account.provenance?.as_of_ms ?? account.fetched_at_ms ?? null,
    by_asset: rows,
    gross_observable_value_usd: gross,
    net_observable_value_usd: net,
    concentration_hhi: gross > 0
      ? rows.reduce((sum, row) => sum + Math.pow(Math.abs(row.net_value_usd) / gross, 2), 0)
      : 0,
    missing_price_symbols: [...new Set(missingPrices)],
    assumptions: [
      "Known USD stablecoins are valued at par and explicitly labeled as an assumption.",
      "Linear perp value uses size × current mid; reported position value is a fallback when a mid is absent.",
      "This is observable exposure, not a canonical total-equity calculation.",
    ],
  };
}

function calculateLiquidationBuffers(snapshot: JsonObject) {
  const positions = Array.isArray(snapshot.account?.positions) ? snapshot.account.positions : [];
  return {
    as_of_ms: snapshot.account?.provenance?.as_of_ms ?? snapshot.account?.fetched_at_ms ?? null,
    rows: positions.map((position: JsonObject) => {
      const size = numericOrZero(position.size);
      const market = preferredMarket(snapshot, position.coin);
      const mid = finiteNumber(market?.mid);
      const liquidation = finiteNumber(position.liquidation_price);
      const bufferPct = mid === null || liquidation === null || mid <= 0
        ? null
        : size >= 0
          ? (mid - liquidation) / mid * 100
          : (liquidation - mid) / mid * 100;
      return {
        coin: position.coin,
        side: size >= 0 ? "long" : "short",
        size,
        market_symbol: market?.symbol ?? null,
        mid,
        liquidation_price: liquidation,
        buffer_pct: bufferPct,
        formula: size >= 0 ? "(mid - liquidation_price) / mid × 100" : "(liquidation_price - mid) / mid × 100",
      };
    }),
    position_count: positions.length,
    positions_complete: snapshot.account?.coverage?.positions?.endpoint_fetch_complete ?? null,
  };
}

function calculateRisk(snapshot: JsonObject) {
  const raw = snapshot?._tool_data?.risk ?? { available: false };
  if (!raw.available) return raw;
  const clearing = raw.clearinghouse ?? {};
  const accountValue = finiteNumber(clearing.account_value);
  const marginUsed = finiteNumber(clearing.total_margin_used);
  const maintenance = finiteNumber(clearing.cross_maintenance_margin_used);
  const notional = finiteNumber(clearing.total_position_notional);
  const portfolioValue = finiteNumber(raw.portfolio_latest?.account_value);
  const spot = (Array.isArray(raw.spot_balances) ? raw.spot_balances : []).map((balance: JsonObject) =>
    valueSpotBalance(snapshot, balance),
  );
  const observableSpotValue = spot.reduce(
    (sum: number, row: JsonObject) => sum + (finiteNumber(row.value_usd) ?? 0),
    0,
  );
  return {
    ...raw,
    deterministic_metrics: {
      clearinghouse_margin_utilization_pct:
        accountValue !== null && accountValue > 0 && marginUsed !== null ? marginUsed / accountValue * 100 : null,
      clearinghouse_maintenance_utilization_pct:
        accountValue !== null && accountValue > 0 && maintenance !== null ? maintenance / accountValue * 100 : null,
      clearinghouse_gross_leverage:
        accountValue !== null && accountValue > 0 && notional !== null ? notional / accountValue : null,
      observable_spot_value_usd: observableSpotValue,
      portfolio_minus_clearinghouse_account_value:
        portfolioValue !== null && accountValue !== null ? portfolioValue - accountValue : null,
    },
    spot_valuations: spot,
    interpretation:
      "Clearinghouse, spot, and portfolio-history values are reported separately. A nonzero reconciliation residual is not automatically a data defect.",
  };
}

async function postJson(url: string, body: unknown, headers: Record<string, string> = {}) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", "user-agent": "Kerosene/Assistant", ...headers },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) throw new Error(`read-only data provider returned HTTP ${response.status}`);
  return response.json();
}

async function fetchCandles(
  symbol: string,
  interval: string,
  startMs: number,
  endMs: number,
): Promise<JsonObject[]> {
  const payload = await postJson(HYPERLIQUID_INFO_URL, {
    type: "candleSnapshot",
    req: { coin: symbol, interval, startTime: startMs, endTime: endMs },
  });
  if (!Array.isArray(payload)) throw new Error("candle provider returned an invalid payload");
  return payload
    .map((row) => ({
      open_time_ms: finiteNumber(row?.t),
      close_time_ms: finiteNumber(row?.T),
      open: finiteNumber(row?.o),
      high: finiteNumber(row?.h),
      low: finiteNumber(row?.l),
      close: finiteNumber(row?.c),
      volume: finiteNumber(row?.v),
    }))
    .filter((row) =>
      row.open_time_ms !== null &&
      row.close_time_ms !== null &&
      row.open !== null &&
      row.high !== null &&
      row.low !== null &&
      row.close !== null &&
      row.volume !== null,
    )
    .sort((left, right) => left.open_time_ms - right.open_time_ms);
}

function summarizeReturns(rows: Array<{ key: string; return_pct: number }>) {
  const groups = new Map<string, { key: string; sample_count: number; total: number; wins: number }>();
  for (const row of rows) {
    const group = groups.get(row.key) ?? { key: row.key, sample_count: 0, total: 0, wins: 0 };
    group.sample_count += 1;
    group.total += row.return_pct;
    if (row.return_pct > 0) group.wins += 1;
    groups.set(row.key, group);
  }
  return [...groups.values()].map((group) => ({
    label: group.key,
    sample_count: group.sample_count,
    average_return_pct: group.sample_count ? group.total / group.sample_count : 0,
    win_rate_pct: group.sample_count ? group.wins / group.sample_count * 100 : 0,
  }));
}

function dateParts(timestamp: number, timeZone: string) {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(timestamp));
  const values = Object.fromEntries(parts.map((part) => [part.type, part.value]));
  return {
    year: Number(values.year),
    month: Number(values.month),
    day: Number(values.day),
    hour: Number(values.hour),
    minute: Number(values.minute),
    second: Number(values.second),
  };
}

function localTimeToUtc(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  timeZone: string,
) {
  const desired = Date.UTC(year, month - 1, day, hour, minute);
  let guess = desired;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const actual = dateParts(guess, timeZone);
    const represented = Date.UTC(
      actual.year,
      actual.month - 1,
      actual.day,
      actual.hour,
      actual.minute,
      actual.second,
    );
    guess += desired - represented;
  }
  return guess;
}

function sessionBoundaries(startMs: number, endMs: number) {
  const specs = [
    { key: "Asia", timeZone: "Asia/Tokyo", hour: 9, minute: 0 },
    { key: "London", timeZone: "Europe/London", hour: 8, minute: 0 },
    { key: "New York", timeZone: "America/New_York", hour: 9, minute: 30 },
    { key: "Overnight", timeZone: "America/New_York", hour: 16, minute: 0 },
  ];
  const boundaries: Array<{ key: string; timestamp: number }> = [];
  for (let cursor = startMs - 3 * 86_400_000; cursor <= endMs + 3 * 86_400_000; cursor += 86_400_000) {
    const utc = new Date(cursor);
    const year = utc.getUTCFullYear();
    const month = utc.getUTCMonth() + 1;
    const day = utc.getUTCDate();
    for (const spec of specs) {
      boundaries.push({
        key: spec.key,
        timestamp: localTimeToUtc(year, month, day, spec.hour, spec.minute, spec.timeZone),
      });
    }
  }
  return boundaries
    .filter((boundary) => boundary.timestamp >= startMs - 86_400_000 && boundary.timestamp <= endMs + 86_400_000)
    .sort((left, right) => left.timestamp - right.timestamp);
}

function sessionSummaries(candles: JsonObject[], startMs: number, endMs: number) {
  const boundaries = sessionBoundaries(startMs, endMs);
  const returns: Array<{ key: string; return_pct: number }> = [];
  for (let index = 0; index + 1 < boundaries.length; index += 1) {
    const start = boundaries[index];
    const end = boundaries[index + 1];
    if (start.timestamp < startMs || end.timestamp > endMs) continue;
    const rows = candles.filter(
      (row) => row.open_time_ms >= start.timestamp && row.open_time_ms < end.timestamp,
    );
    if (!rows.length || rows[0].open <= 0) continue;
    const returnPct = (rows.at(-1)!.close - rows[0].open) / rows[0].open * 100;
    if (Number.isFinite(returnPct)) returns.push({ key: start.key, return_pct: returnPct });
  }
  const order = ["Asia", "London", "New York", "Overnight"];
  return summarizeReturns(returns).sort((left, right) => order.indexOf(left.label) - order.indexOf(right.label));
}

async function fetchPositioning(symbol: string, timeframe: string) {
  const apiKey = process.env.KEROSENE_AGENT_HYPERDASH_API_KEY?.trim();
  if (!apiKey) return { symbol, available: false, reason: "hyperdash_api_key_not_configured" };
  const aggregateQuery = `query KeroseneAggregate($coin: String!) {
    analytics {
      perpsTickerPositions(coin: $coin, limit: 1, offset: 0, side: "all") {
        coin totalLongNotional totalShortNotional totalNotional longCount shortCount totalCount hasMore timestamp
      }
    }
  }`;
  const deltaQuery = `query KeroseneDeltas($market: String!, $timeframe: DeltaTimeframe!) {
    perpDeltas(market: $market, timeframe: $timeframe) { market timeframe deltas { current delta } }
  }`;
  const headers = { authorization: `Bearer ${apiKey}` };
  try {
    const [aggregateRaw, deltaRaw] = await Promise.all([
      postJson(
        HYPERDASH_API_URL,
        { operationName: "KeroseneAggregate", variables: { coin: symbol }, query: aggregateQuery },
        headers,
      ),
      postJson(
        HYPERDASH_API_URL,
        { operationName: "KeroseneDeltas", variables: { market: symbol, timeframe }, query: deltaQuery },
        headers,
      ),
    ]);
    const aggregate = aggregateRaw?.data?.analytics?.perpsTickerPositions;
    const deltas = deltaRaw?.data?.perpDeltas?.deltas;
    if (!aggregate) throw new Error("aggregate unavailable");
    const safeDeltas = Array.isArray(deltas) ? deltas : [];
    return {
      symbol,
      available: true,
      source: "hyperdash_aggregate_only",
      aggregate: {
        coin: aggregate.coin,
        total_long_notional: finiteNumber(aggregate.totalLongNotional),
        total_short_notional: finiteNumber(aggregate.totalShortNotional),
        total_notional: finiteNumber(aggregate.totalNotional),
        long_count: finiteNumber(aggregate.longCount),
        short_count: finiteNumber(aggregate.shortCount),
        total_count: finiteNumber(aggregate.totalCount),
        has_more: Boolean(aggregate.hasMore),
        timestamp: aggregate.timestamp ?? null,
      },
      changes: {
        timeframe,
        wallet_count: safeDeltas.length,
        net_delta: safeDeltas.reduce((sum, row) => sum + numericOrZero(row.delta), 0),
        gross_delta: safeDeltas.reduce((sum, row) => sum + Math.abs(numericOrZero(row.delta)), 0),
        net_current: safeDeltas.reduce((sum, row) => sum + numericOrZero(row.current), 0),
      },
      privacy: "Wallet addresses, labels, and individual rows were neither requested for aggregate positioning nor returned to the model.",
    };
  } catch {
    return { symbol, available: false, reason: "hyperdash_aggregate_request_failed" };
  }
}

export default function keroseneExtension(pi: ExtensionAPI) {
  pi.registerTool({
    name: "kerosene_data",
    label: "Kerosene snapshot",
    description: "Read one public section of the current sanitized Kerosene snapshot. Prefer a narrow section; use all only for a true cross-component summary.",
    promptSnippet: "Read current public Kerosene account, portfolio, market, positioning, and session state",
    promptGuidelines: [
      "Use this tool before claims about current Kerosene state.",
      "Read coverage and provenance fields literally: endpoint completeness and Assistant truncation are different concepts.",
      "Use the deterministic calculation and targeted lookup tools for math or symbol-specific questions.",
    ],
    parameters: Type.Object({
      section: Type.Union(PUBLIC_SECTION_INPUTS.map((name) => Type.Literal(name))),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const payload = params.section === "all"
        ? publicSnapshot(snapshot)
        : {
            schema_version: snapshot.schema_version,
            generated_at_ms: snapshot.generated_at_ms,
            data_policy: snapshot.data_policy,
            [params.section]: snapshot[params.section],
          };
      return toolPayload(payload, { section: params.section });
    },
  });

  pi.registerTool({
    name: "kerosene_market_data",
    label: "Kerosene market lookup",
    description: "Resolve up to 20 raw, canonical, or display symbols to targeted Kerosene market rows with current mids, metadata, and timestamps.",
    promptSnippet: "Look up canonical/raw market mappings and current mids for named symbols",
    promptGuidelines: [
      "Use this instead of reading the capped markets list for symbol-specific work.",
      "Never guess what an @N or #N identifier means; use returned metadata.",
    ],
    parameters: Type.Object({
      symbols: Type.Array(Type.String({ minLength: 1, maxLength: 80 }), {
        minItems: 1,
        maxItems: MAX_MARKET_SYMBOLS,
      }),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const results = params.symbols.map((query: string) => ({
        query,
        matches: resolveMarkets(snapshot, query),
      }));
      return toolPayload({
        as_of_ms: snapshot?._tool_data?.markets?.as_of_ms ?? null,
        results,
        requested_count: params.symbols.length,
        full_market_coverage: snapshot?._tool_data?.markets?.coverage ?? null,
      }, { symbols: params.symbols.length });
    },
  });

  pi.registerTool({
    name: "kerosene_activity",
    label: "Kerosene account activity",
    description: "Read or deterministically aggregate sanitized fills or funding with symbol/time filters and bounded pagination.",
    promptSnippet: "Query bounded fill/funding history or server-side-style aggregates",
    promptGuidelines: [
      "Use aggregate mode for totals; do not manually add many rows.",
      "Always report coverage and distinguish returned rows from total available rows.",
    ],
    parameters: Type.Object({
      kind: Type.Union([Type.Literal("fills"), Type.Literal("funding")]),
      mode: Type.Union([Type.Literal("rows"), Type.Literal("aggregate")]),
      symbol: Type.Optional(Type.String({ minLength: 1, maxLength: 80 })),
      start_ms: Type.Optional(Type.Number({ minimum: 0 })),
      end_ms: Type.Optional(Type.Number({ minimum: 0 })),
      cursor: Type.Optional(Type.Integer({ minimum: 0 })),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: MAX_ACTIVITY_ROWS })),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const sourceRows = activityRows(snapshot, params.kind);
      const filtered = filterActivity(sourceRows, params);
      const sourceCoverage = snapshot?._tool_data?.activity?.coverage?.[params.kind] ?? null;
      if (params.mode === "aggregate") {
        const aggregate = params.kind === "fills" ? aggregateFills(filtered) : aggregateFunding(filtered);
        return toolPayload({
          kind: params.kind,
          mode: params.mode,
          filter: { symbol: params.symbol ?? null, start_ms: params.start_ms ?? null, end_ms: params.end_ms ?? null },
          aggregate,
          coverage: {
            matched_rows: filtered.length,
            source: sourceCoverage,
            aggregate_covers_all_matched_rows: true,
          },
        }, { kind: params.kind, mode: params.mode });
      }
      const cursor = params.cursor ?? 0;
      const limit = params.limit ?? 50;
      const rows = filtered.slice(cursor, cursor + limit);
      const nextCursor = cursor + rows.length < filtered.length ? cursor + rows.length : null;
      return toolPayload({
        kind: params.kind,
        mode: params.mode,
        rows,
        coverage: coverage(rows.length, filtered.length, {
          cursor,
          next_cursor: nextCursor,
          source: sourceCoverage,
        }),
      }, { kind: params.kind, mode: params.mode });
    },
  });

  pi.registerTool({
    name: "kerosene_calculate",
    label: "Kerosene deterministic analysis",
    description: "Run allowlisted deterministic calculations over sanitized Kerosene data: exposure, liquidation buffers, stress, fills, funding, or reconciliation.",
    promptSnippet: "Use deterministic formulas for Kerosene arithmetic instead of mental aggregation",
    promptGuidelines: [
      "Prefer this tool for financial arithmetic and quote its formulas, assumptions, and coverage.",
      "Do not replace a null result with an inferred number.",
    ],
    parameters: Type.Object({
      operation: Type.Union([
        Type.Literal("exposure"),
        Type.Literal("liquidation_buffers"),
        Type.Literal("stress"),
        Type.Literal("fill_aggregation"),
        Type.Literal("funding_aggregation"),
        Type.Literal("portfolio_reconciliation"),
      ]),
      symbol: Type.Optional(Type.String({ minLength: 1, maxLength: 80 })),
      start_ms: Type.Optional(Type.Number({ minimum: 0 })),
      end_ms: Type.Optional(Type.Number({ minimum: 0 })),
      shock_pct: Type.Optional(Type.Number({ minimum: -50, maximum: 50 })),
      include_stablecoins_in_shock: Type.Optional(Type.Boolean()),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      let result: unknown;
      if (params.operation === "exposure") result = calculateExposure(snapshot);
      else if (params.operation === "liquidation_buffers") result = calculateLiquidationBuffers(snapshot);
      else if (params.operation === "fill_aggregation") {
        const rows = filterActivity(activityRows(snapshot, "fills"), params);
        result = {
          aggregate: aggregateFills(rows),
          coverage: { matched_rows: rows.length, source: snapshot?._tool_data?.activity?.coverage?.fills ?? null },
        };
      } else if (params.operation === "funding_aggregation") {
        const rows = filterActivity(activityRows(snapshot, "funding"), params);
        result = {
          aggregate: aggregateFunding(rows),
          coverage: { matched_rows: rows.length, source: snapshot?._tool_data?.activity?.coverage?.funding ?? null },
        };
      } else if (params.operation === "portfolio_reconciliation") result = calculateRisk(snapshot);
      else {
        const exposure = calculateExposure(snapshot);
        const shockPct = params.shock_pct ?? 5;
        const rows = exposure.by_asset.filter(
          (row: JsonObject) => params.include_stablecoins_in_shock || !STABLECOINS.has(normalizedSymbol(row.coin)),
        );
        result = {
          shock_pct: shockPct,
          include_stablecoins: Boolean(params.include_stablecoins_in_shock),
          delta_equity_usd: rows.reduce(
            (sum: number, row: JsonObject) => sum + numericOrZero(row.net_value_usd) * shockPct / 100,
            0,
          ),
          shocked_assets: rows,
          formula: "delta_equity_usd = Σ(net_observable_value_usd × shock_pct / 100)",
          exclusions: params.include_stablecoins_in_shock ? [] : ["known USD stablecoins"],
          exposure_coverage: exposure,
        };
      }
      return toolPayload({ operation: params.operation, result }, { operation: params.operation });
    },
  });

  pi.registerTool({
    name: "kerosene_risk",
    label: "Kerosene portfolio-margin risk",
    description: "Read the canonical, sanitized portfolio-margin risk inputs and deterministic ratios without collapsing clearinghouse, spot, portfolio, and income scopes.",
    promptSnippet: "Reconcile portfolio-margin account value, collateral, maintenance, borrow/supply, and current state",
    promptGuidelines: [
      "Treat each reported scope separately and call a residual a scope difference, not automatically a defect.",
      "Do not claim collateral haircuts or liabilities that are not returned.",
    ],
    parameters: Type.Object({}),
    async execute() {
      const snapshot = await readSnapshot();
      return toolPayload(calculateRisk(snapshot), { section: "risk" });
    },
  });

  pi.registerTool({
    name: "kerosene_positioning",
    label: "Kerosene aggregate positioning",
    description: "Fetch bounded aggregate HyperDash positioning and aggregate change statistics for up to three perp symbols without exposing wallet-level identities.",
    promptSnippet: "Fetch on-demand aggregate long/short positioning and recent changes",
    promptGuidelines: [
      "Never infer crowding when available is false.",
      "This tool returns only aggregates; do not claim wallet-level attribution.",
    ],
    parameters: Type.Object({
      symbols: Type.Array(Type.String({ minLength: 1, maxLength: 80 }), {
        minItems: 1,
        maxItems: MAX_POSITIONING_SYMBOLS,
      }),
      timeframe: Type.Optional(Type.Union([
        Type.Literal("FIFTEEN_MINUTES"),
        Type.Literal("ONE_HOUR"),
        Type.Literal("FOUR_HOURS"),
      ])),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const timeframe = params.timeframe ?? "ONE_HOUR";
      const results = await Promise.all(params.symbols.map(async (query: string) => {
        const market = preferredMarket(snapshot, query);
        if (!market || market.market_type !== "perp") {
          return { query, available: false, reason: "perp_symbol_not_resolved" };
        }
        return { query, ...(await fetchPositioning(market.symbol, timeframe)) };
      }));
      return toolPayload({ timeframe, results }, { symbols: params.symbols.length });
    },
  });

  pi.registerTool({
    name: "kerosene_ohlcv",
    label: "Kerosene OHLCV",
    description: "Fetch bounded read-only Hyperliquid OHLCV for one resolved perp/spot symbol and an allowlisted interval.",
    promptSnippet: "Fetch bounded candle history for trend, volatility, and scenario context",
    promptGuidelines: [
      "Report the exact symbol, interval, requested window, returned count, and source.",
      "Do not describe this as all-history data; the window and row count are bounded.",
    ],
    parameters: Type.Object({
      symbol: Type.String({ minLength: 1, maxLength: 80 }),
      interval: Type.Union(Object.keys(CANDLE_INTERVAL_MS).map((value) => Type.Literal(value))),
      start_ms: Type.Optional(Type.Number({ minimum: 0 })),
      end_ms: Type.Optional(Type.Number({ minimum: 0 })),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: MAX_CANDLES })),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const market = preferredMarket(snapshot, params.symbol);
      if (!market || market.market_type === "outcome") {
        return toolPayload({ available: false, reason: "perp_or_spot_symbol_not_resolved", query: params.symbol });
      }
      const intervalMs = CANDLE_INTERVAL_MS[params.interval];
      const endMs = Math.min(params.end_ms ?? Date.now(), Date.now());
      const limit = params.limit ?? 200;
      const defaultWindow = intervalMs * Math.min(limit, MAX_CANDLES);
      const startMs = Math.max(params.start_ms ?? endMs - defaultWindow, endMs - MAX_CANDLE_LOOKBACK_MS);
      if (endMs <= startMs) return toolPayload({ available: false, reason: "invalid_time_range" });
      try {
        const allRows = await fetchCandles(market.symbol, params.interval, startMs, endMs);
        const rows = allRows.slice(-limit);
        return toolPayload({
          available: true,
          source: "hyperliquid_candleSnapshot",
          symbol: market.symbol,
          canonical_symbol: market.canonical_symbol,
          interval: params.interval,
          requested_start_ms: startMs,
          requested_end_ms: endMs,
          rows,
          coverage: coverage(rows.length, allRows.length, { provider_window_bounded: true }),
        }, { symbol: market.symbol, interval: params.interval });
      } catch {
        return toolPayload({ available: false, reason: "ohlcv_request_failed", symbol: market.symbol });
      }
    },
  });

  pi.registerTool({
    name: "kerosene_sessions",
    label: "Kerosene session statistics",
    description: "Compute on-demand weekday and Kerosene market-session return statistics from bounded Hyperliquid daily and 30-minute candles, independent of open UI panes.",
    promptSnippet: "Compute sample-counted weekday and market-session returns for one symbol",
    promptGuidelines: [
      "Weight conclusions by sample count and report the bounded lookback.",
      "Do not substitute account fills or portfolio buckets for market-session return data.",
    ],
    parameters: Type.Object({
      symbol: Type.String({ minLength: 1, maxLength: 80 }),
      lookback_days: Type.Optional(Type.Union([
        Type.Literal(7),
        Type.Literal(28),
        Type.Literal(56),
        Type.Literal(90),
      ])),
    }),
    async execute(_toolCallId, params) {
      const snapshot = await readSnapshot();
      const market = preferredMarket(snapshot, params.symbol);
      if (!market || market.market_type === "outcome") {
        return toolPayload({ available: false, reason: "perp_or_spot_symbol_not_resolved", query: params.symbol });
      }
      const lookbackDays = params.lookback_days ?? 28;
      const endMs = Date.now();
      const startMs = endMs - lookbackDays * 86_400_000;
      try {
        const [daily, intraday] = await Promise.all([
          fetchCandles(market.symbol, "1d", startMs, endMs),
          fetchCandles(market.symbol, "30m", startMs, endMs),
        ]);
        const weekdayOrder = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        const weekdayRows = daily
          .filter((row) => row.open > 0)
          .map((row) => ({
            key: weekdayOrder[new Date(row.open_time_ms).getUTCDay()],
            return_pct: (row.close - row.open) / row.open * 100,
          }));
        return toolPayload({
          available: true,
          source: "hyperliquid_candleSnapshot_computed_by_kerosene",
          symbol: market.symbol,
          canonical_symbol: market.canonical_symbol,
          lookback_days: lookbackDays,
          requested_start_ms: startMs,
          requested_end_ms: endMs,
          daily_sample_count: daily.length,
          intraday_sample_count: intraday.length,
          weekday_summaries: summarizeReturns(weekdayRows),
          market_session_summaries: sessionSummaries(intraday, startMs, endMs),
          session_definition: "Asia 09:00 Tokyo, London 08:00 London, New York 09:30 New York, Overnight 16:00 New York until next Asia open; DST-aware.",
        }, { symbol: market.symbol, lookback_days: lookbackDays });
      } catch {
        return toolPayload({ available: false, reason: "session_data_request_failed", symbol: market.symbol });
      }
    },
  });

  pi.on("before_agent_start", async (event) => ({
    systemPrompt:
      event.systemPrompt +
      `\n\nYou are the Kerosene trading-data assistant. You can explain, compare, and calculate, but you cannot trade or mutate Kerosene. Use only Kerosene's typed tools for application facts. Start with the narrowest decisive tool; do not call extra sections after a complete empty-state result. Use kerosene_calculate or kerosene_activity aggregate mode for arithmetic. Never guess raw @N/#N symbol mappings. Treat provenance, timestamps, coverage, and truncation fields as authoritative. Distinguish current empty state, unavailable data, and historical activity. Clearinghouse, spot, portfolio, and income fields have different scopes; do not call a residual a defect without evidence. Never imply that you placed, changed, or cancelled an order. Do not provide individualized investment instructions; frame outputs as analytical information. Always finish with visible answer text. Use ordinary Markdown formulas and fenced code, not LaTeX delimiters.`,
  }));
}
