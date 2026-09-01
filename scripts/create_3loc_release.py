#!/usr/bin/env python3

import argparse
import hashlib
import json
import pathlib
import re


ASSETS = (
    "herdr-linux-x86_64",
    "herdr-linux-aarch64",
    "herdr-macos-x86_64",
    "herdr-macos-aarch64",
    "herdr-windows-x86_64.zip",
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--artifacts", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    cargo = pathlib.Path("Cargo.toml").read_text()
    version = re.search(r'^version = "([^"]+)"', cargo, re.MULTILINE)
    protocol = re.search(
        r"pub const PROTOCOL_VERSION: u32 = (\d+);",
        pathlib.Path("src/protocol/wire.rs").read_text(),
    )
    if version is None or protocol is None:
        raise SystemExit("error: could not read version or protocol")

    urls = {}
    checksums = {}
    for name in ASSETS:
        path = args.artifacts / name
        if not path.is_file():
            raise SystemExit(f"error: missing release asset {name}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        key = name.removeprefix("herdr-").removesuffix(".zip")
        urls[key] = (
            f"https://github.com/{args.repository}/releases/download/{args.tag}/{name}"
        )
        checksums[key] = digest
        (args.artifacts / f"{name}.sha256").write_text(f"{digest}  {name}\n")

    manifest = {
        "version": version.group(1),
        "protocol": int(protocol.group(1)),
        "notes": "### 3LOC fork\n- Sidebar navigation, pane notes, and a full-screen keybinding reference.",
        "assets": urls,
        "sha256": checksums,
    }
    args.output.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
