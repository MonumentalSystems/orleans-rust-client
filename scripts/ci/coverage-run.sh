#!/usr/bin/env bash
# orleans-rust-client coverage run for the uranus gnostr-cloud instance.
#
# Extracted from .gnostr-cloud-coverage.yml so the workflow step stays a
# trivial one-liner — wrkflw mangles complex inline `run:` scripts (the
# `${VAR:-default}` and heredoc constructs trip its docker-run builder).
#
# This repo is dual-language, so we measure BOTH halves and merge them into
# the single cargo-llvm-cov-shaped cov-report.json that /coverage renders:
#   * Rust  — cargo-llvm-cov over the workspace (unit + doc tests).
#   * .NET  — `dotnet test --collect "XPlat Code Coverage"` (coverlet →
#             cobertura), adapted into the same {filename,summary,segments}
#             shape (the WO-16 trick used by hyades).
# We then (a) emit ONE combined wall-badge marker from the merged line totals
# and (b) POST the merged report + packed sources to /api/ci/coverage/upload
# so /coverage gets a per-file drilldown spanning both languages.
#
# HERMETIC by design: like the saturn/hyades coverage workflows this skips the
# live e2e suite (`make coverage` runs `cargo test --include-ignored` + a real
# silo/bridge, which needs a writable runtime + free ports the CI sandbox
# doesn't guarantee). Numbers here are unit-test coverage and will read lower
# than the e2e-inclusive figures quoted in the README.
set -e
WS="${GITHUB_WORKSPACE}"; [ -d "$WS" ] || WS=/github/workspace; cd "$WS"

# Writable HOME / nuget cache inside the ephemeral container (dotnet refuses
# to run with an unwritable HOME). CARGO_HOME/SCCACHE_DIR come from the image.
export HOME=/tmp DOTNET_CLI_HOME=/tmp NUGET_PACKAGES=/tmp/nuget \
       DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 CARGO_BUILD_JOBS=4
echo "dotnet: $(dotnet --version 2>&1)   cargo-llvm-cov: $(cargo llvm-cov --version 2>&1 | head -1)"

export MARKERS="$WS/.gnostr-cloud-ci-output.log"; : > "$MARKERS"
if [ -f "$WS/.gnostr-cloud-ci-context.env" ]; then . "$WS/.gnostr-cloud-ci-context.env"; fi

# ── STABLE per-repo target dir — the actual sccache cache-hit fix ──────────────
# The runner hands every job a fresh per-commit workdir, so the default
# `<workdir>/target` yields a brand-new dependency -L/--extern search path each
# run → a different sccache key every time → 0% Rust hits and a full cold
# rebuild of every registry dep on EVERY coverage run. Coverage is enqueued
# serially per repo (campaign rule: ONE cov job), so a stable per-repo dir
# under the host-mounted /cache (ariel runner) is safe and makes the bulk of
# the build — the registry-dependency compiles — hit sccache across runs (and
# lets cargo's own fingerprint DB persist). CARGO_INCREMENTAL=0 because sccache
# cannot cache incremental compilation. Mirrors the canonical gc-uranus
# coverage-run.sh (/cache/cov-target/<slug>).
REPO_SLUG="$(git remote get-url origin 2>/dev/null | sed -E 's#\?.*$##; s#/+$##; s#.*/##; s#\.git$##')"
[ -n "$REPO_SLUG" ] || REPO_SLUG="$(basename "$PWD")"
export CARGO_TARGET_DIR="/cache/cov-target/${REPO_SLUG}"
export CARGO_INCREMENTAL=0   # sccache cannot cache incremental compilation
mkdir -p "$CARGO_TARGET_DIR"
echo "coverage: stable CARGO_TARGET_DIR=$CARGO_TARGET_DIR (sccache-cacheable across runs)"

# ── Rust: cargo-llvm-cov ─────────────────────────────────────────────────────
if ! command -v cargo-llvm-cov >/dev/null; then
  echo "::error::cargo-llvm-cov not on PATH in the runner image" >&2; exit 127
fi
# sccache pre-flight — confirms the host cache mount took (stats=0 forever is
# otherwise indistinguishable from sccache being silently disabled). Route via
# tempfile: piping sccache to `head` makes it panic with "Broken pipe".
sccache --start-server >/dev/null 2>&1 || true
sccache --show-stats > /tmp/sccache-pre.txt 2>&1 || true
echo "sccache pre-build:"; grep -E "^(Compile|Cache hits|Cache misses|Cache location|Max cache size|Version)" /tmp/sccache-pre.txt || head -8 /tmp/sccache-pre.txt

cargo llvm-cov clean --workspace
# No --include-ignored: the ignored tests are the live e2e (silo+bridge).
cargo llvm-cov --workspace --all-features --no-fail-fast --no-report -- --test-threads=1 || \
  echo "::warning::some rust tests failed; emitting coverage from collected data"

sccache --show-stats > /tmp/sccache-post.txt 2>&1 || true
echo "sccache post-build:"; grep -E "^(Compile|Cache hits|Cache misses|Cache hits rate|Cache write)" /tmp/sccache-post.txt || head -10 /tmp/sccache-post.txt

# Same ignore-regex as the Makefile's coverage-rust target (drops generated
# proto, build scripts, the CLI shim, and test code).
RUST_COV_IGNORE='(/target/|build\.rs|/tests/|src/main\.rs|generated\.rs)'
cargo llvm-cov report --json --ignore-filename-regex "$RUST_COV_IGNORE" > /tmp/rust-cov-report.json
echo "rust report: $(jq -r '.data[0].files | length' /tmp/rust-cov-report.json) files, $(jq -r '.data[0].totals.lines.percent' /tmp/rust-cov-report.json)% lines"

# ── .NET: dotnet test + coverlet (cobertura) ─────────────────────────────────
# The .NET build does NOT invoke protoc — the gRPC/protobuf C# sources are
# committed under Generated/ (see Makefile `proto`), so the protoc-less runner
# image is fine. coverlet.collector is referenced by the test csproj.
rm -rf /tmp/cov && mkdir -p /tmp/cov
dotnet test dotnet/OrleansRustBridge.Tests/OrleansRustBridge.Tests.csproj -c Release \
  --collect:"XPlat Code Coverage" \
  --results-directory /tmp/cov \
  --logger "console;verbosity=minimal" || \
  echo "::warning::some .NET tests failed; emitting coverage from collected data"

COB=$(find /tmp/cov -name "*.cobertura.xml" | head -1)
[ -n "$COB" ] && echo "cobertura: $COB" || echo "::warning::no .NET cobertura produced — report will be Rust-only" >&2

# ── Merge Rust + .NET → cov-report.json (+ sources) and emit combined % ──────
python3 - "$COB" "$WS" /tmp/rust-cov-report.json /tmp/cov-report.json /tmp/cov-sources.json "$MARKERS" <<'PY'
import sys, os, json, base64, re
import xml.etree.ElementTree as ET

cob_path, workspace, rust_report, report_out, sources_out, markers = sys.argv[1:7]
PER_FILE_CAP = 1024 * 1024          # 1 MiB/file
TOTAL_CAP    = 24 * 1024 * 1024     # 24 MiB total packed sources

files = []   # merged [{filename, summary, segments}]

# --- Rust half: already in the right shape -----------------------------------
# Stamp each file with language="rust" so the merged report can drive the
# /coverage combined↔separate toggle. Tagging at the SOURCE (here for Rust, in
# the cobertura adapter for C#) is reliable; path/extension inference is not.
try:
    rust = json.load(open(rust_report))
    rust_files = rust.get("data", [{}])[0].get("files", [])
    for f in rust_files:
        f["language"] = "rust"
    files.extend(rust_files)
except (OSError, ValueError, IndexError) as e:
    print(f"::warning::could not load rust report: {e}", file=sys.stderr)

# --- .NET half: cobertura → segments (WO-16 adapter, from hyades) ------------
def adapt_cobertura(path):
    root = ET.parse(path).getroot()
    source_roots = [s.text for s in root.findall("sources/source") if s.text] or [workspace]

    # Build a path-suffix → on-disk index so build-time paths that don't exist
    # at the container's /github/workspace mount still resolve by longest tail.
    ws_index = {}
    if workspace and os.path.isdir(workspace):
        for dp, _, fns in os.walk(workspace):
            if any(seg in dp for seg in ("/bin/", "/obj/", "/.git/", "/node_modules/", "/.nuget/", "/target/")):
                continue
            for fn in fns:
                full = os.path.join(dp, fn)
                ws_index.setdefault(os.path.relpath(full, workspace), full)
                ws_index.setdefault(fn, full)

    def resolve(fn):
        if os.path.isabs(fn) and os.path.exists(fn): return fn
        for r in source_roots:
            p = os.path.join(r, fn)
            if os.path.exists(p): return p
        p = os.path.join(workspace, fn)
        if os.path.exists(p): return p
        parts = fn.lstrip("/").split(os.sep)
        for cut in range(len(parts)):
            key = os.sep.join(parts[cut:])
            if key in ws_index: return ws_index[key]
        return ws_index.get(os.path.basename(fn))

    # cobertura emits one <class> per C# type, so a .cs file with N types shows
    # up N times. Dedup by resolved file, merging per-line hits (covered wins).
    line_hits, methods = {}, {}
    for cls in root.findall(".//classes/class"):
        fn = cls.get("filename", "")
        if not fn: continue
        # Skip generated proto + test code so the merge mirrors the Rust ignore.
        if re.search(r'(Generated/|\.Tests/|/obj/|/bin/)', fn): continue
        abs_fn = resolve(fn) or os.path.join(workspace, fn)
        lh = line_hits.setdefault(abs_fn, {})
        for ln in cls.findall("lines/line"):
            try:
                n, h = int(ln.get("number", "0")), int(ln.get("hits", "0"))
            except ValueError:
                continue
            prev = lh.get(n)                       # `is None` (not `or`): keep h=0 first sightings
            if prev is None or h > prev: lh[n] = h
        ms = methods.setdefault(abs_fn, [])
        for m in cls.findall("methods/method"):
            ms.append(any(int(l.get("hits", "0")) > 0 for l in m.findall("lines/line")))

    out = []
    for abs_fn, lh in line_hits.items():
        lc, lcov = len(lh), sum(1 for h in lh.values() if h > 0)
        ms = methods.get(abs_fn, [])
        mc, mcov = len(ms), sum(1 for c in ms if c)
        lpct = (lcov / lc * 100) if lc else 0.0
        out.append({
            "filename": abs_fn,
            "language": "csharp",
            "summary": {
                "lines":     {"count": lc, "covered": lcov, "percent": lpct},
                "regions":   {"count": lc, "covered": lcov, "percent": lpct},
                "functions": {"count": mc, "covered": mcov, "percent": (mcov/mc*100) if mc else 0.0},
                "branches":  {"count": 0, "covered": 0, "percent": 0.0},
            },
            "segments": [[n, 1, h, True, False, False] for n, h in sorted(lh.items())],
        })
    return out

if cob_path and os.path.exists(cob_path):
    try:
        files.extend(adapt_cobertura(cob_path))
    except Exception as e:                          # noqa: BLE001 — never let .NET parsing kill the Rust report
        print(f"::warning::cobertura adapt failed: {e}", file=sys.stderr)

# --- Recompute combined totals over the merged file set ----------------------
def s(f, k, field): return f["summary"][k][field]
tot = {k: {"count": sum(s(f, k, "count") for f in files),
           "covered": sum(s(f, k, "covered") for f in files)}
       for k in ("lines", "regions", "functions", "branches")}
for k, v in tot.items():
    v["percent"] = (v["covered"] / v["count"] * 100) if v["count"] else 0.0

# Per-language line rollup for the /coverage combined↔separate toggle. Emitted
# ONLY when >1 language is present, so single-language repos produce a report
# byte-identical to before (no by_language key). Each entry sums that language's
# files; the combined % is sum(covered)/sum(total), never a mean of percentages.
by_lang = {}
for f in files:
    lang = f.get("language") or "unknown"
    acc = by_lang.setdefault(lang, {"count": 0, "covered": 0})
    acc["count"]   += s(f, "lines", "count")
    acc["covered"] += s(f, "lines", "covered")
data0 = {"files": files, "totals": tot}
if len(by_lang) > 1:
    data0["by_language"] = {
        lang: {"count": a["count"], "covered": a["covered"],
               "percent": (a["covered"] / a["count"] * 100) if a["count"] else 0.0}
        for lang, a in sorted(by_lang.items())
    }

json.dump({"data": [data0]}, open(report_out, "w"))

# --- Pack source files for the drilldown -------------------------------------
sources, packed_total = {}, 0
oversize = total_cap = missing = 0
for f in files:
    fn = f["filename"]
    if not os.path.exists(fn): missing += 1; continue
    try:
        size = os.path.getsize(fn)
        if size > PER_FILE_CAP: oversize += 1; continue
        if packed_total + size > TOTAL_CAP: total_cap += 1; continue
        sources[fn] = base64.b64encode(open(fn, "rb").read()).decode()
        packed_total += size
    except OSError:
        missing += 1
json.dump(sources, open(sources_out, "w"))

pct = tot["lines"]["percent"]
msg = (f"merged {len(files)} files — lines {tot['lines']['covered']}/{tot['lines']['count']} "
       f"({pct:.2f}%); sources packed {len(sources)} ({packed_total}B) "
       f"oversize={oversize} total_cap={total_cap} missing={missing}")
print(f"::notice::{msg}", file=sys.stderr)
with open(markers, "a") as m:
    m.write(f"##gnostr-cloud-coverage:{pct:.2f}##\n")
    m.write(f"##gnostr-cloud-coverage-adapter:{msg}##\n")
# Emit the wall-badge marker to STDOUT too — the runner scans stdout for
# ##gnostr-cloud-coverage:NN## (the markers FILE alone was NOT picked up, so the
# wall % stayed empty despite a clean 98.49% run). Mirrors hyades/gnostr-cloud.
print(f"##gnostr-cloud-coverage:{pct:.2f}##")
print(f"COMBINED LINE COVERAGE: {pct:.2f}%")
PY

# ── Upload the drilldown report + sources (NON-FATAL) ────────────────────────
# The wall-badge % comes from the inline marker above (already emitted), so a
# CAS-upload hiccup must not fail the job and paint the wall red when coverage
# actually ran. The agent ships JOB_ID + UPLOAD_TOKEN in the context env but
# not the host — default to the LAN NodePort (runner is on dgx-00).
HOST="${GNOSTR_CLOUD_CI_HOST:-http://192.168.1.187:30081}"
if [ -z "$GNOSTR_CLOUD_CI_COVERAGE_UPLOAD_TOKEN" ]; then
  echo "::warning::coverage CAS upload skipped — upload token unset" >&2; exit 0
fi
UPLOAD="${HOST%/}/api/ci/coverage/upload"
AUTH="Authorization: Bearer ${GNOSTR_CLOUD_CI_COVERAGE_UPLOAD_TOKEN}"
up() { # up <kind> <file>
  local rc; rc=$(curl -sS --max-time 90 -o /tmp/up-$1.out -w '%{http_code}' \
    -X POST -H "$AUTH" -H 'Content-Type: application/json' \
    --data-binary @"$2" "${UPLOAD}?kind=$1" 2>/tmp/up-$1.err || echo 000)
  case "$rc" in
    200|201|204) echo "coverage CAS upload ($1): HTTP $rc OK" ;;
    *) echo "::warning::coverage CAS upload ($1) HTTP $rc — $(head -c 200 /tmp/up-$1.out 2>/dev/null)$(head -c 200 /tmp/up-$1.err 2>/dev/null)" >&2 ;;
  esac
}
up report  /tmp/cov-report.json
up sources /tmp/cov-sources.json
exit 0
