#!/usr/bin/env python3
"""Dedicated Xvfb desktop with real XTEST input and process sampling.

Append JSON actions to OUTPUT/commands.jsonl; never connects to the user's display.
Requires Python Pillow/psutil, Xvfb, libX11 and libXtst.
"""
import argparse
import ctypes as C
import json
import os
from pathlib import Path
import select
import re
import subprocess
import time

import psutil
from PIL import ImageGrab

ROOT = Path(__file__).resolve().parents[2]


class Desktop:
    def __init__(self, display):
        self.x = C.CDLL("libX11.so.6")
        self.xt = C.CDLL("libXtst.so.6")
        self.x.XOpenDisplay.argtypes = [C.c_char_p]
        self.x.XOpenDisplay.restype = C.c_void_p
        self.d = self.x.XOpenDisplay(display.encode())
        if not self.d:
            raise RuntimeError("Cannot open isolated display")
        signatures = {
            "XDefaultRootWindow": ([C.c_void_p], C.c_ulong),
            "XStringToKeysym": ([C.c_char_p], C.c_ulong),
            "XKeysymToKeycode": ([C.c_void_p, C.c_ulong], C.c_uint),
            "XFlush": ([C.c_void_p], C.c_int),
            "XSetInputFocus": ([C.c_void_p, C.c_ulong, C.c_int, C.c_ulong], C.c_int),
            "XRaiseWindow": ([C.c_void_p, C.c_ulong], C.c_int),
            "XMoveResizeWindow": ([C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint], C.c_int),
        }
        for name, (args, result) in signatures.items():
            f = getattr(self.x, name)
            f.argtypes, f.restype = args, result
        self.xt.XTestFakeKeyEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.xt.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
        self.xt.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
        self.display = display

    def key_event(self, name, down):
        symbol = self.x.XStringToKeysym(name.encode())
        keycode = self.x.XKeysymToKeycode(self.d, symbol)
        if not keycode:
            raise ValueError(f"Unknown key {name}")
        self.xt.XTestFakeKeyEvent(self.d, keycode, down, 0)

    def key(self, names):
        for name in names:
            self.key_event(name, True)
        for name in reversed(names):
            self.key_event(name, False)
        self.x.XFlush(self.d)

    def action(self, a, output):
        kind = a["action"]
        if kind == "key":
            self.key(a["keys"])
        elif kind == "type":
            for char in a["text"]:
                self.key(["space" if char == " " else char])
                time.sleep(a.get("delay", 0.07))
        elif kind in ("click", "move", "scroll", "down", "up"):
            self.xt.XTestFakeMotionEvent(self.d, -1, a["x"], a["y"], 0)
            if kind != "move":
                for _ in range(a.get("count", 1)):
                    button = a.get("button", 1)
                    if kind != "up":
                        self.xt.XTestFakeButtonEvent(self.d, button, True, 0)
                    if kind != "down":
                        self.xt.XTestFakeButtonEvent(self.d, button, False, 0)
            self.x.XFlush(self.d)
        elif kind in ("focus", "resize"):
            window = str(a["window"])
            if window.startswith("0x") or window.isdecimal():
                wid = int(window, 0)
            else:
                tree = subprocess.check_output(["xwininfo", "-root", "-tree", "-display", self.display], text=True)
                matches = re.findall(r'(0x[0-9a-f]+) "' + re.escape(window) + r'"', tree)
                if len(matches) != 1:
                    raise ValueError(f"Expected one window named {window}")
                wid = int(matches[0], 0)
            self.x.XRaiseWindow(self.d, wid)
            self.x.XSetInputFocus(self.d, wid, 1, 0)
            if kind == "resize":
                self.x.XMoveResizeWindow(self.d, wid, a.get("x", 0), a.get("y", 0), a["width"], a["height"])
            self.x.XFlush(self.d)
        elif kind == "capture":
            time.sleep(0.3)
            label = a["label"]
            if not label.replace("-", "").replace("_", "").isalnum():
                raise ValueError("Invalid screenshot label")
            ImageGrab.grab(xdisplay=self.display).save(output / f"{label}.png")
            tree = subprocess.check_output(["xwininfo", "-root", "-tree", "-display", self.display], text=True)
            (output / f"{label}-windows.txt").write_text(tree)
        elif kind == "phase":
            pass
        elif kind == "pause":
            time.sleep(min(max(a.get("seconds", 0.3), 0), 5))
        else:
            raise ValueError(f"Unknown action {kind}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--seconds", type=int, default=600)
    parser.add_argument("--fixture", type=Path, help="Local server session.json; recommended")
    parser.add_argument("--live", action="store_true", help="Explicitly enable public live traffic")
    parser.add_argument("--no-idle-receiver", action="store_true", help="Use the audit-only comparison binary")
    args = parser.parse_args()
    if bool(args.fixture) == args.live:
        parser.error("Select exactly one of --fixture or --live")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    output.chmod(0o700)
    read_fd, write_fd = os.pipe()
    xvfb = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1600x1000x24", "-nolisten", "tcp"],
                            pass_fds=[write_fd], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.close(write_fd)
    app = None
    try:
        if not select.select([read_fd], [], [], 10)[0]:
            raise RuntimeError("Xvfb startup timed out")
        display = ":" + os.read(read_fd, 100).decode().strip()
        if display == ":":
            raise RuntimeError("Xvfb failed to start")
        desk = Desktop(display)
        # Explicit allowlist: no inherited API credentials, proxy or user config.
        env = {k: v for k, v in os.environ.items() if k in ("PATH", "LANG", "LD_LIBRARY_PATH", "XDG_RUNTIME_DIR")}
        env.update(DISPLAY=display, WINIT_UNIX_BACKEND="x11", WGPU_BACKEND="vulkan",
                   XDG_CONFIG_HOME=str(output / "config"), KEROSENE_AUDIT_LOG=str(output / "telemetry.jsonl"))
        if args.fixture:
            fixture = json.loads(args.fixture.read_text())
            env.update(KEROSENE_AUDIT_HTTP=fixture["http"], KEROSENE_AUDIT_WS=fixture["ws"])
        with (output / "app.log").open("w") as logs:
            binary = "kerosene-e2e-no-idle-receiver" if args.no_idle_receiver else "kerosene-e2e"
            app = subprocess.Popen([str(ROOT / "target/release" / binary), "--test"], env=env, stdout=logs, stderr=logs)
        proc = psutil.Process(app.pid)
        proc.cpu_percent()
        commands = output / "commands.jsonl"
        commands.touch()
        (output / "session.json").write_text(json.dumps({"display": display, "pid": app.pid, "started": time.time(),
            "renderer_request": "vulkan", "test_mode": True, "screen": [1600, 1000]}))
        print(str(output), flush=True)
        deadline = time.monotonic() + args.seconds
        phase = "startup"
        with commands.open() as incoming, (output / "actions.jsonl").open("w") as actions, (output / "process.jsonl").open("w") as metrics:
            while time.monotonic() < deadline and app.poll() is None:
                line = incoming.readline()
                if line:
                    a = json.loads(line)
                    stamp = time.time()
                    if a["action"] == "quit":
                        break
                    if a["action"] == "phase":
                        phase = a["label"]
                    if a["action"] == "fault":
                        if not args.fixture:
                            raise ValueError("Fault injection requires a fixture server")
                        control = args.fixture.resolve().parent / "control.json"
                        temporary = control.with_suffix(".tmp")
                        temporary.write_text(json.dumps(a["settings"]))
                        temporary.replace(control)
                    else:
                        desk.action(a, output)
                    actions.write(json.dumps({"ts": stamp, "completed": time.time(), **a}) + "\n")
                    actions.flush()
                else:
                    time.sleep(0.1)
                metrics.write(json.dumps({"ts":time.time(), "phase":phase, "cpu_pct":proc.cpu_percent(),
                    "rss":proc.memory_info().rss,"threads":proc.num_threads(),"fds":proc.num_fds()}) + "\n")
                metrics.flush()
        (output / "exit.json").write_text(json.dumps({"natural_exit": app.poll(), "ended": time.time()}))
    finally:
        os.close(read_fd)
        if app is not None and app.poll() is None:
            app.terminate()
            try:
                app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()
        xvfb.terminate()
        xvfb.wait(timeout=5)


if __name__ == "__main__":
    main()
