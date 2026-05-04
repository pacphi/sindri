#!/usr/bin/env python3
"""One-shot migration: registry-core/components/* → ADR-030 lifecycle hook layout.

For each component dir that has flat `*.sh` lifecycle scripts:

  1. Move install.sh / uninstall.sh / upgrade.sh / validate.sh /
     configure.sh / project_init.sh / pre_install.sh / post_install.sh
     into a `scripts/` subdir.
  2. Strip the legacy `install.script.*` block from component.yaml.
  3. Add `capabilities.hooks.<phase>: { sh: scripts/<phase>.sh }` for each
     migrated script.

Idempotent — re-running on already-migrated components is a no-op.

Run with `python3 tools/migrate_hooks_to_adr030.py`. Use git to inspect
the resulting diff before committing.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import yaml

# Phase token (kebab-case, mirrors `Phase::as_str()`) → script filename.
PHASE_FILES: list[tuple[str, str, str]] = [
    # (phase token in capabilities.hooks, kebab key in YAML, source filename)
    ("pre-install", "pre-install", "pre_install.sh"),
    ("install", "install", "install.sh"),
    ("post-install", "post-install", "post_install.sh"),
    ("configure", "configure", "configure.sh"),
    ("validate", "validate", "validate.sh"),
    ("upgrade", "upgrade", "upgrade.sh"),
    ("uninstall", "uninstall", "uninstall.sh"),
    ("project-init", "project-init", "project_init.sh"),
]


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def run(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(args),
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    )


def migrate_one(comp_dir: Path) -> tuple[str, list[str]]:
    """Migrate a single component dir. Returns (status, phases_moved)."""
    yaml_path = comp_dir / "component.yaml"
    if not yaml_path.exists():
        return ("skip-no-yaml", [])

    scripts_dir = comp_dir / "scripts"
    moved: list[str] = []

    for phase_token, _yaml_key, filename in PHASE_FILES:
        flat = comp_dir / filename
        nested = scripts_dir / filename
        if flat.exists() and not nested.exists():
            scripts_dir.mkdir(exist_ok=True)
            try:
                run("git", "mv", filename, f"scripts/{filename}", cwd=comp_dir)
            except subprocess.CalledProcessError:
                # Not in git, or git mv failed — fall back to plain move.
                shutil.move(str(flat), str(nested))
            moved.append(phase_token)
        elif nested.exists():
            moved.append(phase_token)

    if not moved:
        return ("skip-no-scripts", [])

    # Patch component.yaml.
    raw = yaml_path.read_text()
    doc = yaml.safe_load(raw) or {}

    # Drop the legacy install.script block. Leave install: {} so the
    # manifest schema still passes (the Script backend now reads its
    # entry point from capabilities.hooks.install).
    install = doc.get("install")
    if isinstance(install, dict) and "script" in install:
        install.pop("script", None)
        if not install:
            doc["install"] = {}

    # Build / merge the capabilities.hooks block.
    caps = doc.setdefault("capabilities", {})
    hooks = caps.setdefault("hooks", {})
    for phase_token in moved:
        rel = f"scripts/{_filename_for(phase_token)}"
        # Don't clobber an explicit hooks entry the author already wrote.
        existing = hooks.get(phase_token)
        if isinstance(existing, dict) and existing.get("sh"):
            continue
        hooks[phase_token] = {"sh": rel}

    # Round-trip-safe write. yaml.safe_dump preserves nothing fancy but
    # registry-core component.yaml files are mechanical anyway.
    # `allow_unicode=True` keeps real em-dashes / non-ASCII chars in
    # the description fields instead of escaping them to `\uXXXX`.
    new_yaml = yaml.safe_dump(
        doc,
        sort_keys=False,
        default_flow_style=False,
        allow_unicode=True,
    )
    yaml_path.write_text(new_yaml)

    return ("migrated", moved)


def _filename_for(phase_token: str) -> str:
    for tok, _key, fname in PHASE_FILES:
        if tok == phase_token:
            return fname
    raise KeyError(phase_token)


def main() -> int:
    root = repo_root()
    components_root = root / "registry-core" / "components"
    if not components_root.exists():
        print(f"no components dir at {components_root}", file=sys.stderr)
        return 1

    summary = {"migrated": 0, "skip-no-scripts": 0, "skip-no-yaml": 0}
    for entry in sorted(components_root.iterdir()):
        if not entry.is_dir():
            continue
        status, phases = migrate_one(entry)
        summary[status] = summary.get(status, 0) + 1
        if status == "migrated":
            print(f"  {entry.name}: {', '.join(phases)}")
        elif status == "skip-no-scripts":
            pass  # quiet
        else:
            print(f"  {entry.name}: {status}")

    print()
    print(f"migrated: {summary['migrated']}")
    print(f"skipped (no flat scripts): {summary['skip-no-scripts']}")
    print(f"skipped (no yaml):         {summary['skip-no-yaml']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
