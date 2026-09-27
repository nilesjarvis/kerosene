#!/usr/bin/env python3
"""Summarize bounded telemetry without reading app logs or credentials."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path


def lines(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def summarize(run):
    rows = lines(run / "telemetry.jsonl")
    samples = lines(run / "process.jsonl")
    actions = lines(run / "actions.jsonl")
    events = [e for r in rows for e in r["network"]]
    timings = defaultdict(lambda: {"count":0,"total_us":0,"max_us":0,"over_16ms":0,"histogram":[0]*32})
    for row in rows:
        for name, value in row["timings"].items():
            t = timings[name]
            for key in ("count", "total_us", "over_16ms"):
                t[key] += value[key]
            t["max_us"] = max(t["max_us"], value["max_us"])
            t["histogram"] = [a+b for a,b in zip(t["histogram"], value["histogram"])]
    for value in timings.values():
        value["mean_us"] = round(value["total_us"] / max(value["count"],1), 2)
        for p in (50,95,99):
            count = 0
            for index, n in enumerate(value["histogram"]):
                count += n
                if count >= value["count"]*p/100:
                    value[f"p{p}_upper_us"] = 2**index
                    break
        del value["histogram"]
    inflight, peak = 0, 0
    for e in events:
        if e["kind"] == "HttpSend":
            inflight += 1
        elif e["kind"].startswith("Http"):
            inflight -= 1
        peak = max(peak,inflight)
    phases = defaultdict(list)
    for sample in samples:
        phases[sample["phase"]].append(sample)
    process = {}
    for phase, group in phases.items():
        weighted, elapsed = 0, 0
        for a,b in zip(group,group[1:]):
            dt = b["ts"]-a["ts"]
            weighted += dt*b["cpu_pct"]
            elapsed += dt
        process[phase] = {"seconds":round(elapsed,2), "cpu_mean_pct":round(weighted/max(elapsed,.001),2),
            "cpu_max_pct":max(g["cpu_pct"] for g in group),
            "rss_start_mib":round(group[0]["rss"]/2**20,2),"rss_end_mib":round(group[-1]["rss"]/2**20,2),
            "rss_peak_mib":round(max(g["rss"] for g in group)/2**20,2),
            "fds_peak":max(g["fds"] for g in group)}
    sends = [e for e in events if e["kind"]=="HttpSend"]
    return {"duration_s":(rows[-1]["ts"]-rows[0]["ts"])/1000,
        "lost_entries":sum(r["lost_entries"] for r in rows), "totals":rows[-1]["totals"],
        "peak_http_awaiting_headers":peak, "final_state":rows[-1].get("state"),
        "http_operations":dict(Counter(e["operation"] for e in sends)),
        "http_results":dict(Counter(e["kind"] for e in events if e["kind"].startswith("Http") and e["kind"]!="HttpSend")),
        "http_sends_first_second":sum(e["ts"]<rows[0]["ts"]+1000 for e in sends),
        "timings":dict(sorted(timings.items(),key=lambda item:-item[1]["total_us"])),
        "process_by_phase":process, "actions":len(actions),
        "ws_lifecycle":dict(Counter(e["kind"] for e in events if e["kind"] in ("WsConnected","WsDisconnected","WsFailed")))}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    summary = summarize(args.run)
    (args.run / "summary.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps({k:v for k,v in summary.items() if k!="timings"},indent=2))


if __name__ == "__main__":
    main()
