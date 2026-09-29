import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";

export async function checkWorkspaceActions(tools: Map<string, any>, snapshotPath: string) {
  const original = await readFile(snapshotPath, "utf8");
  const snapshot = JSON.parse(original);
  snapshot.workspace = {
    charts: [{
      id: 7,
      drawings: [{ id: 1, style: { locked: false } }, { id: 2, style: { locked: true } }],
      drawing_coverage: { truncated: false },
    }],
    indicator_catalog: [{ id: "sma_20" }],
    drawing_catalog: { types: [{ id: "horizontal_level" }] },
  };
  const controller = new AbortController();
  const actions = [
    {
      tool: "kerosene_set_chart_indicators",
      label: "chart-indicator",
      params: { chart_ids: [7], changes: [{ indicator_id: "sma_20", enabled: true }] },
      type: "set_chart_indicators",
      details: { chart_count: 1, indicator_change_count: 1 },
    },
    {
      tool: "kerosene_manage_chart_drawings",
      label: "chart-drawing",
      params: { operations: [
        { operation: "add", chart_id: 7, drawing: { type: "horizontal_level", price: 100 } },
        { operation: "remove", chart_id: 7, drawing_id: 1 },
      ] },
      type: "manage_chart_drawings",
      details: { drawing_operation_count: 2 },
    },
  ];
  let calls = 0;
  try {
    await writeFile(snapshotPath, JSON.stringify(snapshot));
    for (const action of actions) {
      const tool = tools.get(action.tool);
      const invoke = (response: string | undefined) => tool.execute(
        "correlated-call", action.params, controller.signal, undefined,
        { ui: { input: async (title: string, text: string, options: any) => {
          calls += 1;
          assert.equal(title, "KEROSENE_HOST_ACTION_V1");
          assert.deepEqual(JSON.parse(text), {
            version: 1, tool_call_id: "correlated-call", action: { type: action.type, ...action.params },
          });
          assert.equal(options.signal, controller.signal);
          assert.equal(options.timeout, 15_000);
          return response;
        } } },
      );
      for (const response of [
        { success: true, results: [], persisted: false },
        { success: true, results: [{ status: "already_set" }], extra: "preserved" },
      ]) {
        const result = await invoke(JSON.stringify(response));
        assert.deepEqual(result, {
          content: [{ type: "text", text: JSON.stringify(response) }],
          details: action.details,
        });
      }
      for (const [response, message] of [
        [undefined, `Kerosene did not acknowledge the ${action.label} action`],
        ["", `Kerosene did not acknowledge the ${action.label} action`],
        ["{", `Kerosene returned an invalid ${action.label} acknowledgement`],
        ["null", `Kerosene rejected the ${action.label} action`],
        ["[]", `Kerosene rejected the ${action.label} action`],
        ['{"success":"true"}', `Kerosene rejected the ${action.label} action`],
        ['{"success":false,"error":{"message":"host rejection"}}', "host rejection"],
        ['{"success":false,"error":{"message":""}}', ""],
      ] as const) {
        await assert.rejects(invoke(response), { message });
      }
      const transportError = new Error("fixture transport failure");
      await assert.rejects(tool.execute("call", action.params, undefined, undefined, {
        ui: { input: async () => { throw transportError; } },
      }), (error) => error === transportError);

      const cancelled = new AbortController();
      cancelled.abort();
      process.env.KEROSENE_AGENT_SNAPSHOT = `${snapshotPath}.missing`;
      await assert.rejects(tool.execute("call", action.params, cancelled.signal), {
        message: `The ${action.label} action was cancelled`,
      });
      process.env.KEROSENE_AGENT_SNAPSHOT = snapshotPath;
    }
    assert.equal(calls, 20);

    const indicatorTool = tools.get("kerosene_set_chart_indicators");
    const drawingTool = tools.get("kerosene_manage_chart_drawings");
    const context = { ui: { input: async () => assert.fail("invalid actions must not reach the host") } };
    for (const [tool, params, message] of [
      [indicatorTool, { chart_ids: [8], changes: [{ indicator_id: "bad", enabled: true }] },
        "Chart 8 is not open in the current Kerosene workspace snapshot"],
      [indicatorTool, { chart_ids: [7], changes: [{ indicator_id: "bad", enabled: true }] },
        "Indicator 'bad' is not in the current Kerosene workspace catalog"],
      [drawingTool, { operations: [{ operation: "add", chart_id: 8, drawing: { type: "bad" } }] },
        "Chart 8 is not open in the current Kerosene workspace snapshot"],
      [drawingTool, { operations: [{ operation: "add", chart_id: 7, drawing: { type: "bad" } }] },
        "Drawing type 'bad' is not in the current Kerosene workspace catalog"],
      [drawingTool, { operations: [{ operation: "remove", chart_id: 7, drawing_id: 2 }] },
        "Drawing 2 on chart 7 is locked; unlock it before removal"],
      [drawingTool, { operations: [{ operation: "remove", chart_id: 7, drawing_id: 3 }] },
        "Drawing 3 is not on chart 7 in the current Kerosene workspace snapshot"],
    ] as const) {
      await assert.rejects(tool.execute("call", params, undefined, undefined, context), { message });
    }
    snapshot.workspace.charts[0].drawing_coverage.truncated = true;
    await writeFile(snapshotPath, JSON.stringify(snapshot));
    await assert.rejects(drawingTool.execute("call", {
      operations: [{ operation: "remove", chart_id: 7, drawing_id: 3 }],
    }, undefined, undefined, context), {
      message: "Drawing 3 is not visible in the current truncated snapshot for chart 7",
    });
  } finally {
    process.env.KEROSENE_AGENT_SNAPSHOT = snapshotPath;
    await writeFile(snapshotPath, original);
  }
}
