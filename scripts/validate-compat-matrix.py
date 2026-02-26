#!/usr/bin/env python3
"""Validate that compatibility-matrix.yaml ranges match actual extension versions.

For each non-placeholder CLI series (skips 4.0.x), verifies that every
extension's metadata.version satisfies its declared semver range in the matrix.

Exit codes:
  0 — all checks pass
  1 — one or more validation errors

Dependencies: PyYAML (pip install pyyaml)
"""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print("ERROR: PyYAML is required. Install with: pip install pyyaml", file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# Semver helpers (stdlib only — no packaging dependency)
# ---------------------------------------------------------------------------

_SEMVER_RE = re.compile(
    r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)"
    r"(?:-(?P<pre>[0-9A-Za-z\-.]+))?(?:\+(?P<build>[0-9A-Za-z\-.]+))?$"
)


def parse_semver(version_str: str) -> tuple[int, int, int] | None:
    """Return (major, minor, patch) or None if not valid semver."""
    m = _SEMVER_RE.match(version_str.strip())
    if not m:
        return None
    return int(m.group("major")), int(m.group("minor")), int(m.group("patch"))


def satisfies_range(version_str: str, range_str: str) -> bool:
    """Check if *version_str* satisfies a comma-separated semver range.

    Supported operators: >=, >, <=, <, = (or bare version for exact match).
    Example range: ">=1.0.0,<2.0.0"
    """
    ver = parse_semver(version_str)
    if ver is None:
        return False

    for constraint in range_str.split(","):
        constraint = constraint.strip()
        if not constraint:
            continue

        if constraint.startswith(">="):
            bound = parse_semver(constraint[2:])
            if bound is None or ver < bound:
                return False
        elif constraint.startswith(">"):
            bound = parse_semver(constraint[1:])
            if bound is None or ver <= bound:
                return False
        elif constraint.startswith("<="):
            bound = parse_semver(constraint[2:])
            if bound is None or ver > bound:
                return False
        elif constraint.startswith("<"):
            bound = parse_semver(constraint[1:])
            if bound is None or ver >= bound:
                return False
        elif constraint.startswith("="):
            bound = parse_semver(constraint[1:])
            if bound is None or ver != bound:
                return False
        else:
            bound = parse_semver(constraint)
            if bound is None or ver != bound:
                return False

    return True


# ---------------------------------------------------------------------------
# Project root detection
# ---------------------------------------------------------------------------

def find_project_root() -> Path:
    """Detect project root via git toplevel."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, check=True,
        )
        return Path(result.stdout.strip())
    except (subprocess.CalledProcessError, FileNotFoundError):
        return Path.cwd()


# ---------------------------------------------------------------------------
# Main validation
# ---------------------------------------------------------------------------

PLACEHOLDER_SERIES = {"4.0.x"}


def load_extension_versions(v3_dir: Path) -> dict[str, str]:
    """Return {extension_name: version} from all extension.yaml files."""
    versions: dict[str, str] = {}
    ext_dir = v3_dir / "extensions"
    if not ext_dir.is_dir():
        return versions

    for child in sorted(ext_dir.iterdir()):
        ext_file = child / "extension.yaml"
        if not ext_file.is_file():
            continue
        with open(ext_file) as f:
            data = yaml.safe_load(f)
        if not data or "metadata" not in data:
            continue
        meta = data["metadata"]
        name = meta.get("name", child.name)
        version = meta.get("version")
        if version:
            versions[name] = str(version)

    return versions


def validate(project_root: Path) -> int:
    """Run validation. Returns 0 on success, 1 on errors."""
    v3_dir = project_root / "v3"
    matrix_path = v3_dir / "compatibility-matrix.yaml"

    if not matrix_path.is_file():
        print(f"ERROR: Compatibility matrix not found: {matrix_path}", file=sys.stderr)
        return 1

    with open(matrix_path) as f:
        matrix = yaml.safe_load(f)

    cli_versions = matrix.get("cli_versions", {})
    if not cli_versions:
        print("ERROR: No cli_versions found in compatibility matrix", file=sys.stderr)
        return 1

    ext_versions = load_extension_versions(v3_dir)
    if not ext_versions:
        print("ERROR: No extension.yaml files found", file=sys.stderr)
        return 1

    errors: list[str] = []
    warnings: list[str] = []

    for series, series_data in cli_versions.items():
        if series in PLACEHOLDER_SERIES:
            continue

        compat = series_data.get("compatible_extensions", {})
        if not compat:
            warnings.append(f"[{series}] No compatible_extensions defined")
            continue

        # Check each matrix entry against actual extension version
        for ext_name, semver_range in compat.items():
            if ext_name not in ext_versions:
                warnings.append(
                    f"[{series}] '{ext_name}' in matrix but no extension.yaml found"
                )
                continue

            actual_ver = ext_versions[ext_name]
            if not satisfies_range(actual_ver, semver_range):
                errors.append(
                    f"[{series}] '{ext_name}' version {actual_ver} "
                    f"does NOT satisfy matrix range {semver_range}"
                )

        # Check for extensions missing from this series
        matrix_ext_names = set(compat.keys())
        for ext_name in sorted(ext_versions.keys()):
            if ext_name not in matrix_ext_names:
                warnings.append(
                    f"[{series}] '{ext_name}' has extension.yaml but is missing from matrix"
                )

    # Report results
    if warnings:
        print(f"\n{'='*60}")
        print(f"WARNINGS ({len(warnings)}):")
        print(f"{'='*60}")
        for w in warnings:
            print(f"  ⚠  {w}")

    if errors:
        print(f"\n{'='*60}")
        print(f"ERRORS ({len(errors)}):")
        print(f"{'='*60}")
        for e in errors:
            print(f"  ✗  {e}")
        print(f"\n{'='*60}")
        print("Compatibility matrix validation FAILED")
        print(f"{'='*60}")
        return 1

    ext_count = len(ext_versions)
    series_count = len([s for s in cli_versions if s not in PLACEHOLDER_SERIES])
    print(f"✓ Compatibility matrix valid — {ext_count} extensions, {series_count} CLI series checked")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate compatibility-matrix.yaml against extension versions"
    )
    parser.add_argument(
        "--project-root",
        type=Path,
        default=None,
        help="Project root directory (auto-detected via git if omitted)",
    )
    args = parser.parse_args()

    root = args.project_root or find_project_root()
    sys.exit(validate(root))


if __name__ == "__main__":
    main()
