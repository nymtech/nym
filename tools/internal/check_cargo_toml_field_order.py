#!/usr/bin/env python3
"""Check that [package] fields in Cargo.toml follow canonical ordering."""
import re, sys

ORDER = [
    "name", "description", "version", "authors", "edition", "license",
    "repository", "homepage", "documentation", "rust-version", "readme",
    "publish", "keywords", "exclude", "build", "links", "default-run",
    "resolver",
]

bad = []
for path in sys.argv[1:]:
    fields, in_pkg = [], False
    for line in open(path):
        s = line.strip()
        if s == "[package]":
            in_pkg = True
            continue
        if in_pkg and s.startswith("["):
            break
        if in_pkg:
            m = re.match(r"^(\w[\w-]*)", s)
            if m and "=" in line:
                fields.append(m.group(1))
    unknown = [f for f in fields if f not in ORDER]
    if unknown:
        bad.append((path, f"unknown field(s): {', '.join(unknown)}"))
        continue
    expected = sorted(fields, key=ORDER.index)
    if fields != expected:
        first = next(a for a, b in zip(fields, expected) if a != b)
        bad.append((path, f"first mismatch: '{first}'"))

if bad:
    print("[package] fields out of canonical order:")
    for path, reason in bad:
        print(f"  {path}  ({reason})")
    print("\nCanonical order:")
    for f in ORDER:
        print(f"  - {f}")
    sys.exit(1)
