import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
import unittest

from scripts.create_3loc_release import ASSETS


class ReleaseManifestTest(unittest.TestCase):
    def test_manifest_and_checksum_files_match_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts = pathlib.Path(directory)
            for name in ASSETS:
                (artifacts / name).write_bytes(name.encode())
            output = artifacts / "latest.json"

            subprocess.run(
                [
                    "python3",
                    "scripts/create_3loc_release.py",
                    "--tag",
                    "3loc-test",
                    "--repository",
                    "3loc/herdr",
                    "--artifacts",
                    str(artifacts),
                    "--output",
                    str(output),
                ],
                check=True,
            )

            manifest = json.loads(output.read_text())
            self.assertEqual(set(manifest["assets"]), set(manifest["sha256"]))
            self.assertEqual(len(manifest["assets"]), len(ASSETS))
            for name in ASSETS:
                self.assertTrue((artifacts / f"{name}.sha256").is_file())

    def test_installer_verifies_and_installs_release(self):
        payload = b"3loc-herdr-test"
        digest = hashlib.sha256(payload).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            tools = root / "tools"
            tools.mkdir()
            uname = tools / "uname"
            uname.write_text('#!/bin/sh\n[ "$1" = "-s" ] && echo Linux || echo x86_64\n')
            curl = tools / "curl"
            curl.write_text(
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "output = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\n"
                f"payload = {payload!r}\n"
                f"digest = {digest!r}\n"
                "output.write_text(digest + '  herdr-linux-x86_64\\n') "
                "if output.name.endswith('.sha256') else output.write_bytes(payload)\n"
            )
            uname.chmod(0o755)
            curl.chmod(0o755)
            install_dir = root / "install"
            env = os.environ.copy()
            env["PATH"] = f"{tools}:{env['PATH']}"
            env["HERDR_INSTALL_DIR"] = str(install_dir)

            subprocess.run(["sh", "scripts/install-3loc.sh"], env=env, check=True)

            self.assertEqual((install_dir / "herdr").read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
