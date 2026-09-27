#!/usr/bin/env python3
"""Capture nine unauthenticated metadata reads, once, for offline replay."""
import argparse
import json
from pathlib import Path
import time
import urllib.request

KINDS = ("allPerpMetas", "perpConciseAnnotations", "perpDexs", "spotMeta",
         "outcomeMeta", "outcomeTemplates", "metaAndAssetCtxs", "spotMetaAndAssetCtxs", "allMids")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=False)
    for kind in KINDS:
        request = urllib.request.Request("https://api.hyperliquid.xyz/info", json.dumps({"type":kind}).encode(),
                                         {"Content-Type":"application/json", "User-Agent":"Kerosene-E2E-Public-Fixture"})
        with urllib.request.urlopen(request, timeout=15) as response:
            data = json.load(response)
        (args.directory / f"{kind}.json").write_text(json.dumps(data))
        print(kind, flush=True)
        time.sleep(2)
    (args.directory / "capture.json").write_text(json.dumps({"time":time.time(),"types":KINDS,"authenticated":False}))


if __name__ == "__main__":
    main()
