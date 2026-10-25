#!/usr/bin/env python3
"""Compile selected WASM fixtures together, then run their browser suites."""

import json
import os
from pathlib import Path
import subprocess

from ci_scope import BROWSERS


def main():
    root = Path(__file__).resolve().parent.parent
    suites = json.loads(os.environ["BROWSER_SUITES"])
    if not isinstance(suites, list) or any(suite not in BROWSERS for suite in suites):
        raise ValueError("unknown browser suite selection")
    if not suites:
        return
    command = ["cargo", "build", "--locked", "--target", "wasm32-unknown-unknown"]
    for suite in suites:
        command.extend(["--example", suite.replace("-", "_")])
    subprocess.run(command, cwd=root, check=True)
    environment = dict(os.environ, IDLE_FIXTURES_BUILT="1")
    for suite in suites:
        subprocess.run(["bash", f"scripts/build-{suite}.sh"], cwd=root, env=environment, check=True)
        subprocess.run(["npm", "run", f"test:{suite}:browser"], cwd=root, env=environment, check=True)


if __name__ == "__main__":
    main()
