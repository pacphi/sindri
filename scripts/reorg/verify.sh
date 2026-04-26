#!/usr/bin/env bash
# verify.sh — post-cutover verification (see plan §10).
# Run on a fresh clone after v1/v2/v3/v4/main are pushed.

set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0
pass()  { echo "  ✓ $*"; }
fail()  { echo "  ✗ $*"; fail=1; }

echo "1. Tree shape per branch"
for b in main v1 v2 v3 v4; do
  if git rev-parse --verify "origin/$b" >/dev/null 2>&1; then
    pass "$b exists on remote"
  else
    fail "$b missing on remote"
  fi
done

echo
echo "2. No .github/workflows/ on v* branches"
for b in v1 v2 v3 v4; do
  if git ls-tree -r "origin/$b" -- .github/workflows 2>/dev/null | grep -q .; then
    fail "$b has .github/workflows/ (should live only on main)"
  else
    pass "$b has no workflow files"
  fi
done

echo
echo "3. main has no v*/ directories"
for vd in v1 v2 v3 v4; do
  if git ls-tree origin/main -- "$vd" 2>/dev/null | grep -q .; then
    fail "main contains $vd/"
  else
    pass "main has no $vd/"
  fi
done

echo
echo "4. .gitnexus/ never tracked on any branch"
for b in main v1 v2 v3 v4; do
  if git ls-tree -r "origin/$b" -- .gitnexus 2>/dev/null | grep -q .; then
    fail "$b has .gitnexus/ committed (must be gitignored)"
  else
    pass "$b: .gitnexus/ not committed"
  fi
done

echo
echo "5. No stray * 2 / * 3 paths anywhere"
for b in main v1 v2 v3 v4; do
  if git ls-tree -r --name-only "origin/$b" 2>/dev/null | grep -E ' [0-9]+(/|$)' >/dev/null; then
    fail "$b contains tracked '* N' duplicate paths"
  else
    pass "$b clean of stray dups"
  fi
done

echo
echo "6. Each v* has its own root README/CHANGELOG/LICENSE"
for b in v1 v2 v3 v4; do
  for f in README.md CHANGELOG.md LICENSE; do
    if git ls-tree "origin/$b" -- "$f" 2>/dev/null | grep -q .; then
      pass "$b: $f present"
    else
      fail "$b: $f missing"
    fi
  done
done

echo
echo "7. Cross-version source isolation"
git ls-tree -r --name-only origin/v2 2>/dev/null | grep -E '^(v3/|v4/)' | head -3 && fail "v2 leaks into v3/v4 paths"  || pass "v2 isolated"
git ls-tree -r --name-only origin/v3 2>/dev/null | grep -E '^(v2/|v4/)' | head -3 && fail "v3 leaks into v2/v4 paths"  || pass "v3 isolated"
git ls-tree -r --name-only origin/v4 2>/dev/null | grep -E '^(v2/|v3/)' | head -3 && fail "v4 leaks into v2/v3 paths"  || pass "v4 isolated"

echo
[[ $fail -eq 0 ]] && echo "✅ All verifications passed." || { echo "❌ Verifications failed."; exit 1; }
