#!/usr/bin/env python3

import json
import pathlib
import subprocess
import sys
from collections import defaultdict


def dependency_section(dep):
    kind = dep.get("kind") or "normal"
    section = {
        "normal": "dependencies",
        "dev": "dev-dependencies",
        "build": "build-dependencies",
    }.get(kind, f"{kind}-dependencies")
    target = dep.get("target")
    if target:
        return f"target.{target}.{section}"
    return section


def manifest_member(root, manifest_path):
    manifest_parent = pathlib.Path(manifest_path).resolve().parent
    try:
        return str(manifest_parent.relative_to(root))
    except ValueError:
        return str(manifest_parent)


def publish_status(pkg):
    publish = pkg.get("publish")
    if publish is None:
        return True, "publishable to crates.io"

    if isinstance(publish, list):
        if not publish:
            return False, "publish disabled (`publish = false`)"
        if "crates-io" in publish:
            return True, "publishable to crates.io"
        registries = ", ".join(publish)
        return False, f"publish restricted to non-crates.io registries ({registries})"

    return False, f"unrecognized `publish` setting: {publish!r}"


def main():
    root = pathlib.Path(".").resolve()
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            text=True,
        )
    )
    packages_by_id = {pkg["id"]: pkg for pkg in metadata["packages"]}
    workspace_ids = set(metadata["workspace_members"])
    workspace_packages = [
        packages_by_id[pkg_id] for pkg_id in workspace_ids if pkg_id in packages_by_id
    ]
    workspace_by_name = {pkg["name"]: pkg for pkg in workspace_packages}
    workspace_dir_to_name = {
        str(pathlib.Path(pkg["manifest_path"]).resolve().parent): pkg["name"]
        for pkg in workspace_packages
    }

    package_info = {}
    for pkg in workspace_packages:
        name = pkg["name"]
        member = manifest_member(root, pkg["manifest_path"])
        explicitly_publishable, publish_reason = publish_status(pkg)
        package_info[name] = {
            "pkg": pkg,
            "member": member,
            "explicitly_publishable": explicitly_publishable,
            "publish_reason": publish_reason,
        }

    direct_issues = defaultdict(set)
    workspace_deps = defaultdict(list)

    for name, info in package_info.items():
        pkg = info["pkg"]
        member = info["member"]
        explicitly_publishable = info["explicitly_publishable"]

        if not explicitly_publishable:
            direct_issues[name].add(info["publish_reason"])
            continue

        for field in ("description", "license", "repository"):
            value = pkg.get(field)
            if not isinstance(value, str) or not value.strip():
                direct_issues[name].add(f"missing required field '{field}'")

        for dep in pkg.get("dependencies", []):
            section = dependency_section(dep)
            dep_name = dep["name"]
            dep_source = dep.get("source")

            dep_workspace_name = workspace_by_name.get(dep_name, {}).get("name")
            dep_path = dep.get("path")
            if dep_workspace_name is None and dep_path:
                dep_workspace_name = workspace_dir_to_name.get(
                    str(pathlib.Path(dep_path).resolve())
                )

            if dep_path and dep.get("req") in ("*", ""):
                direct_issues[name].add(
                    f"{section}: path dependency '{dep_name}' has no explicit version ({dep_path})"
                )

            if dep_workspace_name:
                workspace_deps[name].append((dep_workspace_name, section))
                continue

            if dep_source and not dep_source.startswith("registry+"):
                direct_issues[name].add(
                    f"{section}: non-registry dependency '{dep_name}' from '{dep_source}'"
                )

    effective_issues = {}

    def collect_effective_issues(crate_name, stack):
        cached = effective_issues.get(crate_name)
        if cached is not None:
            return cached

        issues = set(direct_issues.get(crate_name, set()))
        stack = stack | {crate_name}

        for dep_name, dep_section in workspace_deps.get(crate_name, []):
            dep_info = package_info[dep_name]
            if not dep_info["explicitly_publishable"]:
                issues.add(
                    f"{dep_section}: depends on non-publishable workspace crate '{dep_name}' ({dep_info['publish_reason']})"
                )
                continue

            if dep_name in stack:
                continue

            dep_issues = collect_effective_issues(dep_name, stack)
            if dep_issues:
                issues.add(
                    f"{dep_section}: depends on blocked workspace crate '{dep_name}'"
                )

        effective_issues[crate_name] = issues
        return issues

    for crate_name in package_info:
        collect_effective_issues(crate_name, set())

    unpublishable = []
    for crate_name, info in sorted(package_info.items()):
        issues = sorted(effective_issues.get(crate_name, set()))
        if not issues:
            continue
        unpublishable.append((crate_name, info["member"], issues))

    print("Publishability report:")
    print(f"- workspace crates inspected: {len(package_info)}")
    print(f"- unpublishable crates: {len(unpublishable)}")

    if unpublishable:
        print("\nUnpublishable crate details:")
        for crate_name, member, issues in unpublishable:
            print(f"- {crate_name} ({member})")
            for issue in issues:
                print(f"  - {issue}")

    blocking = []
    for crate_name, info in package_info.items():
        if not info["explicitly_publishable"]:
            continue
        if effective_issues.get(crate_name):
            blocking.append(crate_name)

    if blocking:
        print("\nPreflight checks failed:")
        print(
            f"- {len(blocking)} crate(s) configured for crates.io publish are currently blocked."
        )
        sys.exit(1)

    print("\nPreflight checks passed.")


if __name__ == "__main__":
    main()
