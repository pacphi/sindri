#!/usr/bin/env python3
"""
validate_registry.py

Validates all component.yaml files under registry-core/components/ and
registry-core/collections/. Produces a structured error/warning report.

Checks
------
- YAML parse validity
- Required fields: metadata.name, metadata.version, metadata.license, platforms
- metadata.name matches the enclosing directory name
- Collections: install must be empty, depends_on must be non-empty
- Atomic: at least one install backend configured
- Binary components: no placeholder checksums
- depends_on references resolve against the known component set
- Warnings: missing metadata.homepage, metadata.description
- Warnings (--auth): components in credentialed categories (tags: cloud,
  ai, ai-dev, mcp) without an `auth:` block — opt out per ADR-026 with the
  comment annotation `# sindri-lint: auth-not-required` at the top of
  component.yaml

Usage
-----
    python3 tools/validate_registry.py [--json] [--auth]

Exit code 0 = no errors; 1 = at least one error.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys
from collections import defaultdict

import yaml

COMP_DIR = pathlib.Path(__file__).parent.parent / "registry-core" / "components"
COLL_DIR = pathlib.Path(__file__).parent.parent / "registry-core" / "collections"

KNOWN_BACKENDS = {
    "mise", "apt", "dnf", "zypper", "pacman", "apk", "brew", "winget",
    "scoop", "npm", "pipx", "cargo", "go-install", "binary", "script",
    "sdkman", "collection",
}

# ADR-026 Phase 3 — `--auth` lint rule.
# Tags whose presence indicates a component historically needs credentials.
AUTH_CREDENTIALED_TAGS = {"cloud", "ai", "ai-dev", "mcp"}
AUTH_LINT_OPTOUT = "# sindri-lint: auth-not-required"


def build_known_addresses() -> dict[str, str]:
    known: dict[str, str] = {}
    for d in COMP_DIR.iterdir():
        if not d.is_dir():
            continue
        yf = d / "component.yaml"
        if not yf.exists():
            continue
        try:
            m = yaml.safe_load(yf.read_text())
            install = m.get("install") or {}
            backend = next((k for k in install if k in KNOWN_BACKENDS and install[k] is not None), None)
            key = f"{backend}:{d.name}" if backend else f"?:{d.name}"
        except Exception:
            key = f"?:{d.name}"
        known[key] = str(yf)

    for d in COLL_DIR.iterdir():
        if d.is_dir():
            known[f"collection:{d.name}"] = str(COLL_DIR / d.name / "component.yaml")

    return known


def auth_lint_check(raw: str, manifest: dict) -> str | None:
    """ADR-026 Phase 3 lint: warn on credentialed-category components without `auth:`.

    Returns a warning string, or None if clean (manifest declares `auth:`,
    component opts out via the marker comment, or the tags don't intersect
    the credentialed-category set).

    Warning-only by contract — callers must NOT treat the return value as an
    error condition.
    """
    head = "\n".join(raw.splitlines()[:8])
    if AUTH_LINT_OPTOUT in head:
        return None
    if manifest.get("auth"):
        return None
    tags = set((manifest.get("metadata") or {}).get("tags") or [])
    matched = tags & AUTH_CREDENTIALED_TAGS
    if not matched:
        return None
    return (
        f"AUTH_MISSING: component is in credentialed category "
        f"(tags: {', '.join(sorted(matched))}) but has no `auth:` block. "
        f"Either declare credentials per ADR-026 or add `{AUTH_LINT_OPTOUT}` "
        f"at the top of component.yaml."
    )


def validate_one(
    yf: pathlib.Path,
    expected_name: str,
    is_collection: bool,
    known: dict[str, str],
    auth_lint: bool = False,
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    raw = yf.read_text()
    try:
        m = yaml.safe_load(raw)
    except Exception as e:
        errors.append(f"YAML_PARSE: {e}")
        return errors, warnings

    meta = m.get("metadata") or {}

    if not meta.get("name"):
        errors.append("MISSING_NAME: metadata.name is required")
    elif meta["name"] != expected_name:
        errors.append(
            f"NAME_MISMATCH: metadata.name='{meta['name']}' but dir='{expected_name}'"
        )

    if not meta.get("version"):
        errors.append("MISSING_VERSION: metadata.version is required")

    if not meta.get("license", "").strip():
        errors.append("MISSING_LICENSE: metadata.license must be a non-empty SPDX identifier")

    if not m.get("platforms"):
        errors.append("EMPTY_PLATFORMS: platforms must not be empty")

    if not meta.get("description", "").strip():
        warnings.append("MISSING_DESCRIPTION: metadata.description is recommended")

    if not meta.get("homepage", ""):
        warnings.append("MISSING_HOMEPAGE: metadata.homepage is recommended")

    install = m.get("install") or {}
    deps = m.get("depends_on") or []

    if is_collection:
        active = [k for k, v in install.items() if v is not None]
        if active:
            errors.append(
                f"COLLECTION_HAS_INSTALL: collections must use `install: {{}}` (found: {', '.join(active)})"
            )
        if not deps:
            errors.append("COLLECTION_EMPTY_DEPS: collections must have at least one depends_on entry")
    else:
        active = [k for k, v in install.items() if v is not None]
        if not active:
            errors.append("MISSING_INSTALL: no install backend configured")
        if len(active) > 1:
            warnings.append(
                f"MULTI_INSTALL: multiple backends set ({', '.join(active)}) — only the first will be used"
            )
        binary = install.get("binary") or {}
        if binary:
            bad = [
                p for p, v in (binary.get("checksums") or {}).items()
                if "placeholder" in str(v)
            ]
            if bad:
                errors.append(
                    f"PLACEHOLDER_CHECKSUMS: {len(bad)} platform(s) still have placeholder checksums "
                    f"({', '.join(bad)}) — run `sindri registry fetch-checksums` to populate"
                )

    for dep in deps:
        dep_addr = dep.split("@")[0]
        if dep_addr not in known:
            errors.append(f"BROKEN_DEP: '{dep}' not found in registry")

    # ADR-026 Phase 3 — auth-aware lint (warning-only, never errors).
    if auth_lint and not is_collection:
        w = auth_lint_check(raw, m)
        if w:
            warnings.append(w)

    return errors, warnings


def run(output_json: bool, auth_lint: bool = False) -> int:
    known = build_known_addresses()

    all_errors: dict[str, list[str]] = defaultdict(list)
    all_warnings: dict[str, list[str]] = defaultdict(list)

    for d in sorted(COMP_DIR.iterdir()):
        if d.is_dir():
            e, w = validate_one(d / "component.yaml", d.name, is_collection=False, known=known, auth_lint=auth_lint)
            if e:
                all_errors[f"components/{d.name}"].extend(e)
            if w:
                all_warnings[f"components/{d.name}"].extend(w)

    for d in sorted(COLL_DIR.iterdir()):
        if d.is_dir():
            e, w = validate_one(COLL_DIR / d.name / "component.yaml", d.name, is_collection=True, known=known, auth_lint=auth_lint)
            if e:
                all_errors[f"collections/{d.name}"].extend(e)
            if w:
                all_warnings[f"collections/{d.name}"].extend(w)

    total_comps = sum(1 for d in COMP_DIR.iterdir() if d.is_dir()) + sum(1 for d in COLL_DIR.iterdir() if d.is_dir())
    total_errors = sum(len(v) for v in all_errors.values())
    total_warnings = sum(len(v) for v in all_warnings.values())

    if output_json:
        report = {
            "scanned": total_comps,
            "total_errors": total_errors,
            "total_warnings": total_warnings,
            "errors": {k: v for k, v in sorted(all_errors.items())},
            "warnings": {k: v for k, v in sorted(all_warnings.items())},
        }
        print(json.dumps(report, indent=2))
    else:
        W = 70
        print(f"\n{'='*W}")
        print(f"  Sindri v4 Registry Validation Report")
        print(f"  {total_comps} components/collections scanned")
        print(f"  {total_errors} error(s)  |  {total_warnings} warning(s)")
        print(f"{'='*W}\n")

        if all_errors:
            print("ERRORS (must fix)\n" + "-" * 50)
            for slug in sorted(all_errors):
                print(f"\n  {slug}")
                for e in all_errors[slug]:
                    print(f"    ✗ {e}")

        if all_warnings:
            print("\nWARNINGS (recommended)\n" + "-" * 50)
            for slug in sorted(all_warnings):
                print(f"\n  {slug}")
                for w in all_warnings[slug]:
                    print(f"    ⚠ {w}")

        print(f"\n{'='*W}")
        print(f"  {len(all_errors)} component(s) with errors, {len(all_warnings)} with warnings")
        print(f"{'='*W}\n")

    return 1 if all_errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Validate sindri v4 registry components")
    parser.add_argument("--json", action="store_true", help="Output JSON instead of text")
    parser.add_argument(
        "--auth",
        action="store_true",
        help="Enable ADR-026 Phase 3 auth-aware lint rule (warning-only). "
        "Warns on components in credentialed categories (cloud, ai, ai-dev, "
        "mcp) without an `auth:` block.",
    )
    args = parser.parse_args()
    sys.exit(run(args.json, args.auth))
