#!/usr/bin/env python3
"""Run the native desktop against a local fixture and verify basic coverage.

The memory scenario is identical for baseline and --no-idle-receiver builds.
No live requests are made by this runner or its fixture server.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from analyze import summarize

HERE = Path(__file__).resolve().parent


def wait(seconds):
    return [{"action":"pause", "seconds":min(5, seconds-i)} for i in range(0,seconds,5)]


def memory_scenario():
    return (wait(5) + [
        {"action":"click","x":800,"y":693},
        {"action":"pause","seconds":1},
        {"action":"focus","window":"Kerosene Trading Terminal"},
        {"action":"click","x":1360,"y":197},
        {"action":"type","text":"hype"},
        {"action":"phase","label":"warmup"},
    ] + wait(10) + [
        {"action":"capture","label":"warmup"},
        {"action":"fault","settings":{"tick_ms":50}},
        {"action":"phase","label":"fast-stream"},
    ] + wait(80) + [
        {"action":"capture","label":"fast-stream"},
        {"action":"fault","settings":{}},
        {"action":"phase","label":"cooldown"},
    ] + wait(20) + [{"action":"capture","label":"finish"},{"action":"quit"}])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("fixtures", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--no-idle-receiver", action="store_true")
    parser.add_argument("--scenario", type=Path, help="JSON action list; default is the memory comparison")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    server_dir = args.output / "server"
    run_dir = args.output / "desktop"
    server = None
    desktop = None
    try:
        with (args.output / "server.log").open("w") as log:
            server = subprocess.Popen([sys.executable,str(HERE/"server.py"),str(args.fixtures),str(server_dir)],stdout=log,stderr=log)
        deadline = time.monotonic()+10
        while not (server_dir/"session.json").exists():
            if server.poll() is not None or time.monotonic()>deadline:
                raise RuntimeError("Fixture server failed to start; see server.log")
            time.sleep(.1)
        command = [sys.executable,str(HERE/"desktop.py"),str(run_dir),"--fixture",str(server_dir/"session.json"),"--seconds","900"]
        if args.no_idle_receiver:
            command.append("--no-idle-receiver")
        desktop = subprocess.Popen(command)
        deadline = time.monotonic()+10
        while not (run_dir/"commands.jsonl").exists():
            if desktop.poll() is not None or time.monotonic()>deadline:
                raise RuntimeError("Desktop failed to start")
            time.sleep(.1)
        actions = json.loads(args.scenario.read_text()) if args.scenario else memory_scenario()
        with (run_dir/"commands.jsonl").open("a") as commands:
            commands.writelines(json.dumps(a)+"\n" for a in actions)
        if desktop.wait(timeout=910) != 0:
            raise RuntimeError("Desktop driver failed")
        summary = summarize(run_dir)
        (run_dir/"summary.json").write_text(json.dumps(summary,indent=2)+"\n")
        required = ["EnterApplication","SymbolsLoaded","SymbolSearchChanged","ChartWsCandleUpdate","WsBookUpdate"]
        checks = {
            "app_survived":json.loads((run_dir/"exit.json").read_text())["natural_exit"] is None,
            "no_telemetry_loss":summary["lost_entries"] == 0,
            "no_account_connected":summary["final_state"]["connected"] is False,
            "history_loaded":summary["final_state"]["candles"] > 1,
            "screenshots_completed":(run_dir/"finish.png").exists(),
            **{f"handled_{name}":name in summary["timings"] for name in required},
        }
        (args.output/"checks.json").write_text(json.dumps(checks,indent=2)+"\n")
        print(json.dumps({"checks":checks,"process":summary["process_by_phase"]},indent=2),flush=True)
        if not all(checks.values()):
            raise SystemExit(1)
    finally:
        for process in (desktop,server):
            if process is not None and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    main()
