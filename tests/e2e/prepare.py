#!/usr/bin/env python3
"""Build an isolated, instrumented copy of the current working source (no secrets)."""
import hashlib
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "target/e2e/source"


def replace_once(text, before, after):
    if text.count(before) != 1:
        raise ValueError(f"Instrumentation anchor changed: {before}")
    return text.replace(before, after, 1)


def main():
    global DEST
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-idle-receiver", action="store_true", help="Audit-only comparison; change HL receiver ownership")
    args = parser.parse_args()
    binary_name = "kerosene-e2e-no-idle-receiver" if args.no_idle_receiver else "kerosene-e2e"
    if args.no_idle_receiver:
        DEST = ROOT / "target/e2e/source-no-idle-receiver"
    DEST.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / "src", DEST / "src", dirs_exist_ok=True)
    for name in ("Cargo.lock", "build.rs"):
        shutil.copy2(ROOT / name, DEST / name)
    for name in ("assets", "packaging"):
        if not (DEST / name).exists():
            (DEST / name).symlink_to(ROOT / name, target_is_directory=True)
    cargo = (ROOT / "Cargo.toml").read_text()
    cargo = replace_once(cargo, "[package]\n", "[package]\nautobins = false\n")
    cargo += f'\n[[bin]]\nname = "{binary_name}"\npath = "src/main.rs"\n'
    (DEST / "Cargo.toml").write_text(cargo)
    main_rs = (ROOT / "src/main.rs").read_text()
    main_rs = replace_once(main_rs, "mod account;", "mod audit_runtime;\nmod account;")
    main_rs = replace_once(main_rs, "    configure_graphics_backend();", "    audit_runtime::start();\n    configure_graphics_backend();")
    for before, after in (("TradingTerminal::update,", "audit_runtime::update,"),
                          ("TradingTerminal::view_window,", "audit_runtime::view,"),
                          (".subscription(TradingTerminal::subscription)", ".subscription(audit_runtime::subscription)")):
        main_rs = replace_once(main_rs, before, after)
    (DEST / "src/main.rs").write_text(main_rs)
    http = (DEST / "src/network_activity/http.rs").read_text()
    http = replace_once(http, "    let result = client.execute(request).await;",
                        "    let mut request = request;\n    crate::audit_runtime::redirect_http(&mut request);\n    let result = client.execute(request).await;")
    (DEST / "src/network_activity/http.rs").write_text(http)
    ws = (DEST / "src/ws/manager.rs").read_text()
    ws = replace_once(ws, "            WS_URL.to_string(),", "            crate::audit_runtime::ws_url(WS_URL),")
    if args.no_idle_receiver:
        ws = replace_once(ws, "    msg_rx: broadcast::Receiver<WsRoutedMessage>,", "    msg_tx: broadcast::Sender<WsRoutedMessage>,")
        ws = replace_once(ws, "let (msg_tx, msg_rx) = broadcast::channel(10000);", "let (msg_tx, _msg_rx) = broadcast::channel(10000);")
        ws = replace_once(ws, "            msg_tx,\n            reconnect_gate,", "            msg_tx.clone(),\n            reconnect_gate,")
        ws = replace_once(ws, "            msg_rx,\n        }", "            msg_tx,\n        }")
        ws = replace_once(ws, "mgr.msg_rx.resubscribe()", "mgr.msg_tx.subscribe()")
    (DEST / "src/ws/manager.rs").write_text(ws)
    message = (ROOT / "src/message.rs").read_text().split("enum Message {", 1)[1].split("\n}", 1)[0]
    arms = []
    for name, delimiter in re.findall(r"^    ([A-Z]\w*)\s*([({,])", message, re.M):
        pattern = {"(": "(..)", "{": " { .. }", ",": ""}[delimiter]
        arms.append(f'        Message::{name}{pattern} => "{name}",')
    if len(arms) < 100:
        raise ValueError("Message enum parser did not find expected variants")
    runtime = (ROOT / "tests/e2e/runtime.rs").read_text()
    runtime += '\nfn message_name(message: &Message) -> &\'static str {\n    match message {\n' + '\n'.join(arms) + '\n    }\n}\n'
    (DEST / "src/audit_runtime.rs").write_text(runtime)
    hashes = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted((ROOT / "src").rglob("*.rs"))}
    for name in ("Cargo.toml", "Cargo.lock", "tests/e2e/runtime.rs"):
        hashes[name] = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
    (DEST.parent / f"{binary_name}-source-manifest.json").write_text(json.dumps({
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "sha256": hashes, "message_variants": len(arms), "experiment":args.no_idle_receiver}, indent=2))
    subprocess.run(["rustfmt", "--edition", "2024", str(DEST / "src/audit_runtime.rs")], check=True)
    subprocess.run(["cargo", "build", "--release", "--locked", "--manifest-path", str(DEST / "Cargo.toml"),
                    "--target-dir", str(ROOT / "target"), "--bin", binary_name], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
