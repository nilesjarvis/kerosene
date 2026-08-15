import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { readFile } from "node:fs/promises";

const SECTION_NAMES = [
  "overview",
  "account",
  "portfolio",
  "markets",
  "positioning",
  "sessions",
  "all",
] as const;

export default function keroseneExtension(pi: ExtensionAPI) {
  pi.registerTool({
    name: "kerosene_data",
    label: "Kerosene data",
    description:
      "Read the current sanitized, read-only Kerosene trading snapshot. Use sections to keep context focused.",
    promptSnippet: "Read current Kerosene account, portfolio, market, positioning, and session data",
    promptGuidelines: [
      "Use kerosene_data before making claims about the user's live Kerosene data.",
      "Treat timestamps and completeness fields from kerosene_data as authoritative and call out stale or missing data.",
    ],
    parameters: Type.Object({
      section: Type.Union(SECTION_NAMES.map((name) => Type.Literal(name)), {
        description: "The snapshot section to read; use all only when multiple sections are necessary.",
      }),
    }),
    async execute(_toolCallId, params) {
      const snapshotPath = process.env.KEROSENE_AGENT_SNAPSHOT;
      if (!snapshotPath) {
        throw new Error("KEROSENE_AGENT_SNAPSHOT is not configured");
      }

      const snapshot = JSON.parse(await readFile(snapshotPath, "utf8"));
      const payload =
        params.section === "all"
          ? snapshot
          : {
              schema_version: snapshot.schema_version,
              generated_at_ms: snapshot.generated_at_ms,
              data_policy: snapshot.data_policy,
              [params.section]: snapshot[params.section],
            };

      return {
        content: [{ type: "text", text: JSON.stringify(payload) }],
        details: { section: params.section },
      };
    },
  });

  pi.on("before_agent_start", async (event) => ({
    systemPrompt:
      event.systemPrompt +
      `\n\nYou are the Kerosene trading-data assistant. You can explain, compare, calculate, and write analysis code or pseudocode, but you cannot trade or mutate Kerosene. Use only the kerosene_data tool for application facts. Never imply that you placed, changed, or cancelled an order. Clearly distinguish observed data from inference. Mention stale, partial, or missing data. Do not provide individualized investment instructions; frame outputs as analytical information.`,
  }));
}
