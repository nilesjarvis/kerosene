#!/usr/bin/env python3
"""Offline HTTP/WS market fixture with file-controlled fault injection.

Only public metadata is replayed. Candles/books/ticks are synthetic; this is not
an exchange emulator. Unknown HTTP operations fail visibly. No upstream traffic.
"""
import argparse
import asyncio
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import math
from pathlib import Path
import threading
import time

import websockets


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("fixtures", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    data = {p.stem: json.loads(p.read_text()) for p in args.fixtures.glob("*.json")}
    control = args.output / "control.json"
    control.write_text("{}")
    lock = threading.Lock()
    activity = (args.output / "requests.jsonl").open("w")
    active = 0
    dex_mids = {"": data["allMids"]}
    dex_contexts = {"": data["metaAndAssetCtxs"]}
    for meta in data["allPerpMetas"][1:]:
        names = [entry["name"] for entry in meta["universe"]]
        if names:
            dex = names[0].split(":", 1)[0]
            dex_mids[dex] = {name:"100" for name in names}
            dex_contexts[dex] = [meta, [{"markPx":"100","midPx":"100","oraclePx":"100","prevDayPx":"100",
                "dayNtlVlm":"100000","funding":"0.00001","openInterest":"1000","premium":"0"} for _ in names]]

    def log(**values):
        with lock:
            activity.write(json.dumps({"ts":time.time(), **values}) + "\n")
            activity.flush()

    def settings():
        try:
            return json.loads(control.read_text())
        except (ValueError, OSError):
            return {}

    def price(coin):
        return float(data["allMids"].get(coin, 0.5 if coin.startswith("#") else 100))

    def candle(coin, interval, stamp):
        px = price(coin)
        return {"t":stamp,"T":stamp+interval-1,"s":coin,"i":"1h","o":str(px),
                "h":str(px*1.002),"l":str(px*0.998),"c":str(px*(1+0.0005*math.sin(stamp/60000))),"v":"100","n":10}

    def interval_ms(name):
        return int(name[:-1]) * {"m":60000,"h":3600000,"d":86400000,"w":604800000,"M":2592000000}[name[-1]]

    def book(coin):
        px = price(coin)
        return {"coin":coin,"time":int(time.time()*1000),"levels":[
            [{"px":str(px*(1-0.0001*i)),"sz":"10","n":2} for i in range(1,21)],
            [{"px":str(px*(1+0.0001*i)),"sz":"10","n":2} for i in range(1,21)]]}

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"
        def log_message(self, *_):
            pass

        def respond(self, status, body):
            payload = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            if status == 429:
                self.send_header("Retry-After", "60")
            self.end_headers()
            try:
                self.wfile.write(payload)
            except (BrokenPipeError, ConnectionResetError):
                pass

        def do_GET(self):
            log(kind="get", path=self.path, status=200 if self.path == "/info" else 501)
            self.respond(200 if self.path == "/info" else 501, {})

        def do_POST(self):
            nonlocal active
            request = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))))
            kind = request.get("type", "unknown")
            with lock:
                active += 1
                concurrency = active
            started = time.time()
            status, body = 200, {}
            try:
                options = settings()
                time.sleep(options.get("delay_ms", 25) / 1000)
                if self.path != "/info":
                    status = 403
                elif options.get("http_status") and kind in options.get("fault_types", [kind]):
                    status = options["http_status"]
                elif options.get("malformed") and kind in options.get("fault_types", [kind]):
                    body = {"deliberately":"wrong shape"}
                elif kind == "ping":
                    body = None
                elif kind == "allMids":
                    body = dex_mids.get(request.get("dex", ""), {})
                elif kind == "metaAndAssetCtxs":
                    body = dex_contexts.get(request.get("dex", ""), [{"universe":[]},[]])
                elif kind in data:
                    body = data[kind]
                elif kind == "candleSnapshot":
                    req = request["req"]
                    interval = interval_ms(req["interval"])
                    end = req["endTime"] // interval * interval
                    start = max(req["startTime"] // interval * interval, end - 4999 * interval)
                    body = [dict(candle(req["coin"], interval, t), i=req["interval"]) for t in range(start,end+1,interval)]
                elif kind == "l2Book":
                    body = book(request["coin"])
                elif kind == "exchangeStatus":
                    body = "normal"
                else:
                    status = 501
                self.respond(status, body)
            finally:
                log(kind=kind, status=status, elapsed_ms=(time.time()-started)*1000, concurrency=concurrency,
                    coin=request.get("coin",request.get("req",{}).get("coin")),
                    interval=request.get("req",{}).get("interval"))
                with lock:
                    active -= 1

    class Server(ThreadingHTTPServer):
        request_queue_size = 1024
        daemon_threads = True

    http = Server(("127.0.0.1", 0), Handler)
    threading.Thread(target=http.serve_forever, daemon=True).start()

    async def stream(socket, *unused):
        subscriptions = {}
        initial_generation = settings().get("disconnect", 0)
        log(kind="ws_connected")

        async def receive():
            async for raw in socket:
                message = json.loads(raw)
                if message.get("method") == "ping":
                    await socket.send(json.dumps({"channel":"pong"}))
                    continue
                sub = message.get("subscription", {})
                key = json.dumps(sub, sort_keys=True)
                if message.get("method") == "subscribe":
                    subscriptions[key] = sub
                else:
                    subscriptions.pop(key, None)
                log(kind="ws_"+message.get("method","unknown"), topic=sub.get("type"), coin=sub.get("coin"))
                await socket.send(json.dumps({"channel":"subscriptionResponse","data":message}))

        receiver = asyncio.create_task(receive())
        try:
            while not receiver.done():
                options = settings()
                if options.get("disconnect", 0) != initial_generation:
                    await socket.close()
                    break
                if not options.get("silent"):
                    for sub in list(subscriptions.values()):
                        kind, coin = sub.get("type"), sub.get("coin","HYPE")
                        body = None
                        if kind == "allMids":
                            body = {"mids":dex_mids.get(sub.get("dex",""),{}), "dex":sub.get("dex","")}
                        elif kind == "l2Book":
                            body = book(coin)
                        elif kind == "candle":
                            interval = interval_ms(sub["interval"])
                            now = int(time.time()*1000)//interval*interval
                            body = dict(candle(coin, interval, now), i=sub["interval"])
                        elif kind == "activeAssetCtx":
                            body = {"coin":coin,"ctx":{"funding":"0.00001","openInterest":"1000","prevDayPx":str(price(coin)),
                                "dayNtlVlm":"10000000","premium":"0","oraclePx":str(price(coin)),"markPx":str(price(coin)),"midPx":str(price(coin))}}
                        if body is not None:
                            await socket.send(json.dumps({"channel":kind,"data":body}))
                await asyncio.sleep(options.get("tick_ms", 1000)/1000)
        finally:
            receiver.cancel()
            await asyncio.gather(receiver, return_exceptions=True)
            log(kind="ws_disconnected")

    async def run():
        async with websockets.serve(stream, "127.0.0.1", 0) as ws:
            port = ws.sockets[0].getsockname()[1]
            (args.output / "session.json").write_text(json.dumps({"http":f"http://127.0.0.1:{http.server_port}","ws":f"ws://127.0.0.1:{port}"}))
            print(str(args.output / "session.json"), flush=True)
            await asyncio.Future()
    asyncio.run(run())


if __name__ == "__main__":
    main()
