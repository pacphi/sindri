#!/usr/bin/env python3
"""
fetch_checksums.py

Fetches real SHA-256 checksums for binary components in registry-core/components/
and writes them into each component.yaml, replacing sha256:placeholder-* values.

Strategies (tried in order per component):
  1. checksums_txt  — project publishes a single {name}_checksums.txt
  2. sidecar        — each asset has a .sha256 file at {asset_url}.sha256
  3. download       — download the binary and compute SHA-256 locally

Usage
-----
    python3 tools/fetch_checksums.py              # all placeholder components
    python3 tools/fetch_checksums.py gh kapp ytt  # specific components
    python3 tools/fetch_checksums.py --dry-run    # print resolved URLs, no writes

Exit 0 → all checksums resolved; 1 → at least one component still has gaps.
"""
from __future__ import annotations

import argparse
import hashlib
import pathlib
import re
import sys
import urllib.error
import urllib.request
from typing import Optional

import yaml

COMP_DIR = pathlib.Path(__file__).parent.parent / "registry-core" / "components"

# ── platform substitution ────────────────────────────────────────────────────
# Maps (sindri_os, sindri_arch) → (url_os, url_arch) for each component.
# None means "this platform is not available as a binary asset" (skip it).
_STD = {  # linux/darwin, amd64/arm64
    ("linux", "x86_64"):  ("linux",  "amd64"),
    ("linux", "aarch64"): ("linux",  "arm64"),
    ("macos", "x86_64"):  ("darwin", "amd64"),
    ("macos", "aarch64"): ("darwin", "arm64"),
}

PLATFORM_SUB: dict[str, dict[tuple, tuple | None]] = {
    "gh": {
        ("linux",   "x86_64"):  ("linux",   "amd64"),
        ("linux",   "aarch64"): ("linux",   "arm64"),
        ("macos",   "x86_64"):  ("macOS",   "amd64"),
        ("macos",   "aarch64"): ("macOS",   "arm64"),
        ("windows", "x86_64"):  ("windows", "amd64"),
    },
    "flyctl": {
        ("linux",  "x86_64"):  ("Linux",  "x86_64"),
        ("linux",  "aarch64"): ("Linux",  "arm64"),
        ("macos",  "x86_64"):  ("macOS",  "x86_64"),
        ("macos",  "aarch64"): ("macOS",  "arm64"),
    },
    # kubectx/kubens keep full x86_64 in their filenames
    "kubectx": {
        ("linux",  "x86_64"):  ("linux",  "x86_64"),
        ("linux",  "aarch64"): ("linux",  "arm64"),
        ("macos",  "x86_64"):  ("darwin", "x86_64"),
        ("macos",  "aarch64"): ("darwin", "arm64"),
    },
    "kubens": {
        ("linux",  "x86_64"):  ("linux",  "x86_64"),
        ("linux",  "aarch64"): ("linux",  "arm64"),
        ("macos",  "x86_64"):  ("darwin", "x86_64"),
        ("macos",  "aarch64"): ("darwin", "arm64"),
    },
    # AWS CLI: macOS uses a .pkg installer, not the zip URL — skip
    "aws-cli": {
        ("linux",  "x86_64"):  ("linux",  "x86_64"),
        ("linux",  "aarch64"): ("linux",  "aarch64"),
        ("macos",  "x86_64"):  None,
        ("macos",  "aarch64"): None,
    },
    # IBM Cloud: macOS name is "osx" in their download URLs
    "ibmcloud": {
        ("linux",  "x86_64"):  ("linux",  "amd64"),
        ("linux",  "aarch64"): ("linux",  "arm64"),
        ("macos",  "x86_64"):  ("osx",    "x86_64"),
        ("macos",  "aarch64"): ("osx",    "arm64"),
    },
    # fabric: assets are fabric_Linux_x86_64.tar.gz / fabric_Darwin_arm64.tar.gz
    "fabric": {
        ("linux",  "x86_64"):  ("Linux",  "x86_64"),
        ("linux",  "aarch64"): ("Linux",  "arm64"),
        ("macos",  "x86_64"):  ("Darwin", "x86_64"),
        ("macos",  "aarch64"): ("Darwin", "arm64"),
    },
    # aliyun macOS uses "macosx" not "darwin"
    "aliyun": {
        ("linux",  "x86_64"):  ("linux",   "amd64"),
        ("linux",  "aarch64"): ("linux",   "arm64"),
        ("macos",  "x86_64"):  ("macosx",  "amd64"),
        ("macos",  "aarch64"): ("macosx",  "arm64"),
    },
    **{
        name: _STD for name in [
            "doctl", "kapp", "kbld", "imgpkg", "vendir", "ytt",
        ]
    },
}

# ── checksum sources ─────────────────────────────────────────────────────────
# Tools that publish a single checksums.txt listing all assets.
# The file contains lines like: "<sha256hex>  <filename>"
CHECKSUMS_TXT: dict[str, str] = {
    "gh":
        "https://github.com/cli/cli/releases/download/v{version}/gh_{version}_checksums.txt",
    "fabric":
        "https://github.com/danielmiessler/fabric/releases/download/v{version}/fabric_{version}_checksums.txt",
    "flyctl":
        "https://github.com/superfly/flyctl/releases/download/v{version}/flyctl_{version}_checksums.txt",
    "doctl":
        "https://github.com/digitalocean/doctl/releases/download/v{version}/doctl-{version}-checksums.txt",
    "kubectx":
        "https://github.com/ahmetb/kubectx/releases/download/v{version}/checksums.txt",
    "kubens":
        "https://github.com/ahmetb/kubectx/releases/download/v{version}/checksums.txt",
}


# ── helpers ──────────────────────────────────────────────────────────────────

def _get(url: str, timeout: int = 30) -> Optional[bytes]:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "sindri-fetch-checksums/1.0"})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.read()
    except (urllib.error.HTTPError, urllib.error.URLError, OSError):
        return None


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _expand(template: str, version: str, os_: str, arch: str) -> str:
    return template.replace("{version}", version).replace("{os}", os_).replace("{arch}", arch)


def _parse_checksums_txt(text: str) -> dict[str, str]:
    """Parse SHA256SUMS-style files: '{hash}  {filename}' or '{hash} {filename}'."""
    result: dict[str, str] = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 2:
            hash_val, filename = parts[0], parts[-1]
            # Strip any leading '*' (binary mode marker)
            filename = filename.lstrip("*")
            if re.fullmatch(r"[0-9a-f]{64}", hash_val):
                result[filename] = hash_val
    return result


# ── per-component resolution ─────────────────────────────────────────────────

def resolve_component(name: str, dry_run: bool) -> dict[str, str]:
    """
    Returns {platform_key: sha256hex} for all platforms where a checksum
    was successfully obtained. Skips platforms mapped to None.
    """
    yf = COMP_DIR / name / "component.yaml"
    if not yf.exists():
        print(f"  [{name}] component.yaml not found — skip")
        return {}

    m = yaml.safe_load(yf.read_text())
    binary = (m.get("install") or {}).get("binary")
    if not binary:
        return {}

    url_template = binary["url_template"]
    version = m["metadata"]["version"]
    platforms = m.get("platforms") or []
    sub_map = PLATFORM_SUB.get(name, _STD)

    # Build (sindri_key, url_os, url_arch) triplets
    targets: list[tuple[str, str, str]] = []
    for p in platforms:
        os_, arch = p["os"], p["arch"]
        sub = sub_map.get((os_, arch))
        if sub is None:
            print(f"  [{name}] skipping {os_}-{arch} (no binary for this platform)")
            continue
        url_os, url_arch = sub
        targets.append((f"{os_}-{arch}", url_os, url_arch))

    results: dict[str, str] = {}

    # ── Strategy 1: checksums.txt ──────────────────────────────────────────
    if name in CHECKSUMS_TXT:
        txt_url = CHECKSUMS_TXT[name].replace("{version}", version)
        print(f"  [{name}] checksums.txt → {txt_url}")
        if not dry_run:
            data = _get(txt_url)
            if data:
                lookup = _parse_checksums_txt(data.decode("utf-8", errors="replace"))
                for key, url_os, url_arch in targets:
                    asset_url = _expand(url_template, version, url_os, url_arch)
                    asset_filename = asset_url.split("/")[-1]
                    # Also try common extension variants (.zip for macOS/Windows binaries)
                    alt_filename = None
                    if asset_filename.endswith(".tar.gz"):
                        alt_filename = asset_filename[:-7] + ".zip"
                    elif asset_filename.endswith(".zip"):
                        alt_filename = asset_filename[:-4] + ".tar.gz"
                    matched = lookup.get(asset_filename) or (alt_filename and lookup.get(alt_filename))
                    if matched:
                        results[key] = matched
                        print(f"    ✓ {key}: {matched[:16]}...")
                    else:
                        print(f"    ✗ {key}: '{asset_filename}' not in checksums.txt")
            else:
                print(f"    ✗ could not fetch checksums.txt")

        if results or dry_run:
            return results  # fall through to sidecar only for gaps

    # ── Strategy 2: per-asset .sha256 sidecar ─────────────────────────────
    for key, url_os, url_arch in targets:
        if key in results:
            continue
        asset_url = _expand(url_template, version, url_os, url_arch)
        sidecar_url = asset_url + ".sha256"
        print(f"  [{name}] sidecar {key} → {sidecar_url}")
        if dry_run:
            continue
        data = _get(sidecar_url)
        if data:
            text = data.decode("utf-8", errors="replace").strip().split()[0]
            if re.fullmatch(r"[0-9a-f]{64}", text):
                results[key] = text
                print(f"    ✓ {key}: {text[:16]}...")
            else:
                print(f"    ✗ {key}: sidecar content not a SHA-256 hash")
        else:
            print(f"    ✗ {key}: sidecar not available")

    # ── Strategy 3: download-and-hash ─────────────────────────────────────
    for key, url_os, url_arch in targets:
        if key in results:
            continue
        asset_url = _expand(url_template, version, url_os, url_arch)
        print(f"  [{name}] downloading {key} → {asset_url}")
        if dry_run:
            continue
        data = _get(asset_url, timeout=120)
        if data:
            h = _sha256(data)
            results[key] = h
            print(f"    ✓ {key}: {h[:16]}... (computed from {len(data)//1024}KB download)")
        else:
            print(f"    ✗ {key}: download failed")

    return results


def update_yaml(name: str, new_checksums: dict[str, str]) -> None:
    if not new_checksums:
        return
    yf = COMP_DIR / name / "component.yaml"
    text = yf.read_text()
    for platform_key, sha256hex in new_checksums.items():
        # Replace placeholder values in-place, preserving YAML formatting
        placeholder = f"sha256:placeholder-{platform_key}"
        real_value = f"sha256:{sha256hex}"
        text = text.replace(placeholder, real_value)
    yf.write_text(text)


def main() -> int:
    parser = argparse.ArgumentParser(description="Fetch real SHA-256 checksums for binary components")
    parser.add_argument("components", nargs="*", help="Component names (default: all with placeholders)")
    parser.add_argument("--dry-run", action="store_true", help="Print resolved URLs without fetching")
    args = parser.parse_args()

    # Collect targets: either named args or all binary components with placeholder checksums
    if args.components:
        targets = args.components
    else:
        targets = []
        for d in sorted(COMP_DIR.iterdir()):
            if not d.is_dir():
                continue
            yf = d / "component.yaml"
            if yf.exists() and "sha256:placeholder" in yf.read_text():
                targets.append(d.name)

    if not targets:
        print("No binary components with placeholder checksums found.")
        return 0

    print(f"\nFetching checksums for {len(targets)} component(s)...\n")
    gaps = 0

    for name in targets:
        print(f"► {name}")
        new = resolve_component(name, dry_run=args.dry_run)
        if not args.dry_run:
            update_yaml(name, new)

        # Count remaining placeholders
        yf = COMP_DIR / name / "component.yaml"
        if yf.exists():
            remaining = yf.read_text().count("sha256:placeholder")
            if remaining:
                print(f"  ⚠ {remaining} platform(s) still have placeholder checksums\n")
                gaps += 1
            else:
                print(f"  ✓ all checksums resolved\n")

    return 1 if gaps else 0


if __name__ == "__main__":
    sys.exit(main())
