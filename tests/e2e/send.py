#!/usr/bin/env python3
"""Append an action or list of actions to an isolated running desktop."""
import json
from pathlib import Path
import sys

actions = json.loads(sys.argv[2])
if isinstance(actions, dict):
    actions = [actions]
with (Path(sys.argv[1]) / "commands.jsonl").open("a") as out:
    for action in actions:
        out.write(json.dumps(action) + "\n")
