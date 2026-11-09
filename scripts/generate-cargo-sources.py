#!/usr/bin/env python3
"""Generate a Flatpak cargo-sources.json from Cargo.lock.

The output follows the format produced by flatpak-builder-tools
(https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo): every
crates.io dependency becomes a pinned archive source plus a .cargo-checksum.json
sidecar, and a trailing cargo config switches the build to vendored sources so
that the Flatpak build never touches the network.

Checksums are taken from Cargo.lock itself (lockfile v3/v4 record them inline),
so the script is fully offline and deterministic. Only registry dependencies
are supported; a git/path dependency outside the workspace fails loudly instead
of silently producing an incomplete source list.

Usage:
    scripts/generate-cargo-sources.py                 # write the file
    scripts/generate-cargo-sources.py --check         # verify it is up to date
    scripts/generate-cargo-sources.py -o /tmp/out.json
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LOCKFILE = REPO_ROOT / "Cargo.lock"
DEFAULT_OUTPUT = REPO_ROOT / "packaging" / "flatpak" / "cargo-sources.json"

CARGO_CONFIG = (
    '[source.vendored-sources]\n'
    'directory = "cargo/vendor"\n'
    "\n"
    "[source.crates-io]\n"
    'replace-with = "vendored-sources"\n'
)


def cargo_sources(lockfile: Path) -> list[dict]:
    with lockfile.open("rb") as fh:
        lock = tomllib.load(fh)

    registry = "registry+https://github.com/rust-lang/crates.io-index"
    sources: list[dict] = []
    for package in sorted(lock["package"], key=lambda p: (p["name"], p["version"])):
        origin = package.get("source")
        if origin is None:
            # Workspace members have no source and are built from the checkout.
            continue
        if not origin.startswith(registry):
            raise SystemExit(
                f"{lockfile}: package {package['name']} {package['version']} "
                f"uses an unsupported source ({origin}). Only crates.io "
                "dependencies can be vendored by this generator."
            )
        checksum = package["checksum"]
        crate_dir = f"cargo/vendor/{package['name']}-{package['version']}"
        crate_url = (
            f"https://static.crates.io/crates/"
            f"{package['name']}/{package['name']}-{package['version']}.crate"
        )
        sources.append(
            {
                "type": "archive",
                "archive-type": "tar-gzip",
                "url": crate_url,
                "sha256": checksum,
                "dest": crate_dir,
            }
        )
        sources.append(
            {
                "type": "inline",
                "contents": json.dumps({"package": checksum, "files": {}}),
                "dest": crate_dir,
                "dest-filename": ".cargo-checksum.json",
            }
        )

    if not sources:
        raise SystemExit(f"{lockfile}: no registry dependencies found.")

    sources.append(
        {
            "type": "inline",
            "contents": CARGO_CONFIG,
            "dest": "cargo",
            "dest-filename": "config",
        }
    )
    return sources


def render(sources: list[dict]) -> str:
    return json.dumps(sources, indent=4) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lockfile", type=Path, default=DEFAULT_LOCKFILE)
    parser.add_argument("-o", "--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail (without writing) if the committed file is out of date",
    )
    args = parser.parse_args()

    rendered = render(cargo_sources(args.lockfile))
    if args.check:
        try:
            committed = args.output.read_text()
        except FileNotFoundError:
            print(f"error: {args.output} is missing; run without --check to create it")
            return 1
        if json.loads(committed) != json.loads(rendered):
            print(
                f"error: {args.output} is out of date. Regenerate it with "
                "scripts/generate-cargo-sources.py after changing Cargo.lock."
            )
            return 1
        print(f"{args.output} is up to date with {args.lockfile}.")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered)
    print(f"wrote {args.output} ({rendered.count(chr(10))} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
