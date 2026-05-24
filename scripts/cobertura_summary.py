#!/usr/bin/env python3
"""Print a per-file line-coverage summary from a Cobertura XML report.

Filters to our hand-written .NET sources: only classes whose package
(assembly) name contains one of --include substrings, excluding generated
gRPC/protobuf files. Usage:

    cobertura_summary.py <report.xml> [--include OrleansRustBridge]
"""

import argparse
import collections
import sys
import xml.etree.ElementTree as ET

GENERATED = {"OrleansBridge.cs", "OrleansBridgeGrpc.cs"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("--include", nargs="*", default=["OrleansRustBridge"])
    parser.add_argument("--exclude", nargs="*", default=["Tests"])
    args = parser.parse_args()

    root = ET.parse(args.report).getroot()
    files = collections.defaultdict(lambda: [set(), set()])  # covered, all (line numbers)

    for package in root.iter("package"):
        name = package.get("name", "")
        if not any(inc in name for inc in args.include):
            continue
        if any(exc in name for exc in args.exclude):
            continue
        for cls in package.iter("class"):
            filename = cls.get("filename", "").replace("\\", "/").split("/")[-1]
            if not filename or filename in GENERATED:
                continue
            for line in cls.iter("line"):
                number = line.get("number")
                files[filename][1].add(number)
                if int(line.get("hits", "0")) > 0:
                    files[filename][0].add(number)

    if not files:
        print("no matching classes found in report", file=sys.stderr)
        return 1

    covered_total = line_total = 0
    print(f"{'COVERAGE':>8}  {'LINES':>9}  FILE")
    for filename in sorted(files):
        covered = len(files[filename][0])
        total = len(files[filename][1])
        covered_total += covered
        line_total += total
        pct = 100 * covered / total if total else 0.0
        print(f"{pct:7.1f}%  {covered:4d}/{total:<4d}  {filename}")

    pct = 100 * covered_total / line_total if line_total else 0.0
    print(f"{'-' * 8}")
    print(f"{pct:7.1f}%  {covered_total:4d}/{line_total:<4d}  TOTAL (hand-written)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
