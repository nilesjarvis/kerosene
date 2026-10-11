// Offline browser-wallet regression harness; no browser, network or wallet used.
// Run: node --test tests/manual/builder-approval.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const ACCOUNT = "0x1111111111111111111111111111111111111111";
const OTHER_ACCOUNT = "0x3333333333333333333333333333333333333333";
const BUILDER = "0x2222222222222222222222222222222222222222";
const SIGNATURE = "0x" + "11".repeat(32) + "22".repeat(32) + "1b";
const EXCHANGE = "https://api.hyperliquid.xyz/exchange";
const html = readFileSync(new URL("../../assets/builder-approval.html", import.meta.url), "utf8");
const match = html.match(/<script nonce="__KEROSENE_NONCE__">([\s\S]*?)<\/script>/);
assert.ok(match, "approval page contains its inline script");

// The Rust schema test independently verifies the server-generated contract.
function configuration() {
  return {
    account: ACCOUNT,
    feeBps: "1.3",
    expiresAt: 1_700_000_300_123,
    completionPath: "/session/complete",
    action: {
      type: "approveBuilderFee", hyperliquidChain: "Mainnet",
      maxFeeRate: "0.013%", builder: BUILDER, nonce: 1_700_000_000_123,
    },
    domain: {
      name: "HyperliquidSignTransaction", version: "1",
      verifyingContract: "0x0000000000000000000000000000000000000000",
    },
    primaryType: "HyperliquidTransaction:ApproveBuilderFee",
    types: {
      EIP712Domain: [
        {name: "name", type: "string"}, {name: "version", type: "string"},
        {name: "chainId", type: "uint256"}, {name: "verifyingContract", type: "address"},
      ],
      "HyperliquidTransaction:ApproveBuilderFee": [
        {name: "hyperliquidChain", type: "string"}, {name: "maxFeeRate", type: "string"},
        {name: "builder", type: "address"}, {name: "nonce", type: "uint64"},
      ],
    },
  };
}

function harness(options = {}) {
  const config = configuration();
  const clock = {now: config.action.nonce};
  const walletCalls = [];
  const fetchCalls = [];
  const timers = [];
  const elements = new Map();
  let signed = false;
  const element = () => ({
    textContent: "", value: "0", disabled: false, listeners: new Map(), children: [],
    addEventListener(name, callback) { this.listeners.set(name, callback); },
    appendChild(child) { this.children.push(child); },
  });
  for (const name of ["account", "builder", "fee", "wallet", "approve", "cancel", "status"]) {
    elements.set(name, element());
  }
  elements.get("approve").disabled = true; // Initial disabled attribute in the HTML.
  const provider = {
    async request(request) {
      walletCalls.push(request);
      switch (request.method) {
        case "eth_requestAccounts": return [options.initialAccount ?? ACCOUNT];
        case "eth_accounts": return [options.finalAccount ?? ACCOUNT];
        case "eth_chainId": return signed ? (options.finalChain ?? "0xa4b1") : (options.initialChain ?? "0xa4b1");
        case "eth_signTypedData_v4":
          if (options.signError) throw options.signError;
          signed = true;
          if (options.expireDuringSigning) clock.now = config.expiresAt;
          return options.signature ?? SIGNATURE;
        default: throw new Error("Unexpected wallet request: " + request.method);
      }
    },
  };
  const context = vm.createContext({
    document: {getElementById: name => elements.get(name), createElement: element},
    window: {
      ethereum: options.noWallet ? undefined : provider,
      addEventListener() {}, dispatchEvent() {},
    },
    Event: class Event {},
    Date: {now: () => clock.now},
    AbortSignal: {timeout: () => undefined},
    setTimeout(callback, duration) {
      if (duration === 300) callback();
      else timers.push(callback);
    },
    async fetch(url, init) {
      const body = JSON.parse(init.body);
      fetchCalls.push({url, init, body});
      if (url === EXCHANGE) {
        if (options.exchangeError) throw new Error("Simulated lost response");
        return {
          ok: options.httpOk ?? true,
          async json() {
            if (options.invalidJson) throw new Error("Invalid JSON");
            return options.exchangeResult ?? {status: "ok", response: {type: "default"}};
          },
        };
      }
      assert.equal(url, config.completionPath, "only the session completion route is called");
      return {ok: true};
    },
  });
  vm.runInContext(match[1].replace("__KEROSENE_CONFIG__", JSON.stringify(config)), context);
  return {
    config, clock, walletCalls, fetchCalls, elements,
    approve: () => elements.get("approve").listeners.get("click")(),
    cancel: () => elements.get("cancel").listeners.get("click")(),
    expire() { clock.now = config.expiresAt; for (const callback of timers) callback(); },
    posts: () => fetchCalls.filter(call => call.url === EXCHANGE),
    completions: () => fetchCalls.filter(call => call.url === config.completionPath).map(call => call.body.status),
  };
}

test("signs the displayed account, exact fee and user-signed schema, then submits once", async () => {
  const app = harness();
  await app.approve();
  const signing = app.walletCalls.find(call => call.method === "eth_signTypedData_v4");
  assert.equal(signing.params[0], ACCOUNT);
  const typedData = JSON.parse(signing.params[1]);
  assert.deepEqual(typedData, {
    domain: {...app.config.domain, chainId: "0xa4b1"},
    types: app.config.types, primaryType: app.config.primaryType, message: app.config.action,
  });
  assert.equal(app.posts().length, 1);
  assert.deepEqual(app.posts()[0].body, {
    action: {...app.config.action, signatureChainId: "0xa4b1"},
    nonce: app.config.action.nonce,
    signature: {r: "0x" + "11".repeat(32), s: "0x" + "22".repeat(32), v: 27},
  });
  assert.equal(app.posts()[0].init.credentials, "omit");
  assert.equal(app.posts()[0].init.redirect, "error");
  assert.deepEqual(app.completions(), ["approved"]);
  await app.approve();
  assert.equal(app.posts().length, 1, "completed sessions cannot submit again");
});

test("wrong initial wallet account cannot sign or submit", async () => {
  const app = harness({initialAccount: OTHER_ACCOUNT});
  await app.approve();
  assert.equal(app.walletCalls.some(call => call.method === "eth_signTypedData_v4"), false);
  assert.equal(app.fetchCalls.length, 0);
  assert.match(app.elements.get("status").textContent, /main account shown above/);
});

test("wallet rejection remains retryable and cancellation releases the session", async () => {
  const app = harness({signError: {code: 4001}});
  await app.approve();
  assert.equal(app.posts().length, 0);
  assert.equal(app.elements.get("approve").disabled, false);
  assert.match(app.elements.get("status").textContent, /cancelled/);
  await app.cancel();
  assert.deepEqual(app.completions(), ["cancelled"]);
});

test("switching account while the wallet signs prevents submission", async () => {
  const app = harness({finalAccount: OTHER_ACCOUNT});
  await app.approve();
  assert.equal(app.posts().length, 0);
  assert.match(app.elements.get("status").textContent, /main account shown above/);
});

test("switching chain while signing prevents submission", async () => {
  const app = harness({finalChain: "0x1"});
  await app.approve();
  assert.equal(app.posts().length, 0);
});

test("a supported current wallet chain can sign without any network switch", async () => {
  const app = harness({initialChain: "0x1", finalChain: "0x1"});
  await app.approve();
  assert.equal(app.posts()[0].body.action.signatureChainId, "0x1");
  assert.equal(app.posts()[0].body.action.hyperliquidChain, "Mainnet");
  assert.equal(app.walletCalls.some(call => call.method.includes("switch")), false);
});

test("malformed signatures and unsupported recovery IDs cannot be posted", async () => {
  for (const signature of ["0x", "0xzz", "0x" + "11".repeat(64), "0x" + "11".repeat(64) + "02"]) {
    const app = harness({signature});
    await app.approve();
    assert.equal(app.posts().length, 0);
  }
});

test("wallet recovery IDs zero and one normalize to 27 and 28", async () => {
  for (const [suffix, expected] of [["00", 27], ["01", 28]]) {
    const app = harness({signature: "0x" + "11".repeat(64) + suffix});
    await app.approve();
    assert.equal(app.posts()[0].body.signature.v, expected);
  }
});

test("submitted failures release pending state for a recheck without resubmission", async () => {
  for (const options of [
    {exchangeError: true}, {httpOk: false}, {invalidJson: true},
    {exchangeResult: {status: "err", response: "Rejected"}},
    {exchangeResult: {status: "ok", response: {type: "unexpected"}}},
  ]) {
    const app = harness(options);
    await app.approve();
    assert.equal(app.posts().length, 1);
    assert.deepEqual(app.completions(), ["submitted"]);
    assert.equal(app.elements.get("approve").disabled, true);
    await app.approve();
    assert.equal(app.posts().length, 1);
  }
});

test("expired sessions never sign or post, including expiration during signing", async () => {
  const app = harness();
  app.expire();
  await app.approve();
  assert.equal(app.walletCalls.length, 0);
  assert.equal(app.posts().length, 0);
  const during = harness({expireDuringSigning: true});
  await during.approve();
  assert.equal(during.posts().length, 0);
  assert.match(during.elements.get("status").textContent, /expired/);
});

test("missing wallet leaves approval disabled", () => {
  const app = harness({noWallet: true});
  assert.equal(app.elements.get("approve").disabled, true);
  assert.match(app.elements.get("status").textContent, /No browser wallet found/);
});
