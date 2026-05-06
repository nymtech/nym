#!/usr/bin/env python3
"""Check [package] field ordering and required fields in Cargo.toml files."""
import re, sys

ORDER = [
    "name", "description", "version", "authors", "edition", "license",
    "repository", "homepage", "documentation", "rust-version", "readme",
    "publish", "keywords", "exclude", "build", "links", "default-run",
    "resolver",
]

# Required when publish = true or publish.workspace = true
REQUIRED = {
    "name", "description", "version", "authors", "edition", "license",
    "repository", "homepage", "documentation", "rust-version", "readme",
    "publish",
}

bad = []
for path in sys.argv[1:]:
    fields, values, in_pkg = [], {}, False
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
                values[m.group(1)] = s

    if not fields:
        continue

    unknown = [f for f in fields if f not in ORDER]
    if unknown:
        bad.append((path, f"unknown field(s): {', '.join(unknown)}"))
        continue

    expected = sorted(fields, key=ORDER.index)
    if fields != expected:
        first = next(a for a, b in zip(fields, expected) if a != b)
        bad.append((path, f"field ordering: first mismatch is '{first}'"))

    # publish must always be present (explicit intent)
    field_set = set(fields)
    if "publish" not in field_set:
        bad.append((path, "missing 'publish' field (must be explicit)"))
    # Check remaining required fields when publishable
    elif "true" in values["publish"] or "workspace" in values["publish"]:
        missing = sorted(REQUIRED - field_set, key=ORDER.index)
        if missing:
            bad.append((path, f"missing required: {', '.join(missing)}"))

if bad:
    print("[package] field issues:")
    for path, reason in bad:
        print(f"  {path}  ({reason})")
    print("\nCanonical order:")
    for f in ORDER:
        req = " (required)" if f in REQUIRED else ""
        print(f"  - {f}{req}")
    sys.exit(1)
