"""Tests for install_local_package.sh using temporary package and link directories."""

import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "install_local_package.sh"
VERSION = "1.0.0+local.1"
EXPECTED_VERSION = f"codex-cli {VERSION}"


def write_executable(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"#!/bin/sh\n{body}\n")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def make_package(
    root: Path, codex_body: str = "", host_body: str = "echo usage"
) -> Path:
    package = root / "package"
    write_executable(
        package / "bin" / "codex", f'echo "{EXPECTED_VERSION}"\n{codex_body}'
    )
    write_executable(package / "bin" / "codex-code-mode-host", host_body)
    (package / "codex-package.json").write_text(f'{{"version": "{VERSION}"}}\n')
    return package


class InstallLocalPackageTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.lib = self.root / "lib"
        self.bin = self.root / "bin-links"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def install(self, package: Path) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["sh", str(SCRIPT), str(package), str(self.lib), str(self.bin)],
            capture_output=True,
            text=True,
            check=False,
        )

    def link_targets(self) -> dict[str, str]:
        return {
            "current": os.readlink(self.lib / "current"),
            "codex": os.readlink(self.bin / "codex"),
            "codex-code-mode-host": os.readlink(self.bin / "codex-code-mode-host"),
        }

    def test_links_resolve_through_current_release(self) -> None:
        package = make_package(self.root)
        result = self.install(package)
        release = self.lib / f"{VERSION}-{self.package_sha(package)}"
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.link_targets(),
            {
                "current": str(release),
                "codex": f"{self.lib}/current/bin/codex",
                "codex-code-mode-host": f"{self.lib}/current/bin/codex-code-mode-host",
            },
        )
        self.assertEqual(
            subprocess.run(
                [str(self.bin / "codex"), "--version"], capture_output=True, text=True
            ).stdout.strip(),
            EXPECTED_VERSION,
        )

    def test_reinstalling_same_package_keeps_links(self) -> None:
        package = make_package(self.root)
        self.assertEqual(self.install(package).returncode, 0)
        first = self.link_targets()
        result = self.install(package)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.link_targets(), first)

    def test_refuses_to_replace_regular_file(self) -> None:
        package = make_package(self.root)
        self.bin.mkdir(parents=True)
        (self.bin / "codex").write_text("user file\n")
        result = self.install(package)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.bin / "codex").read_text(), "user file\n")
        self.assertFalse((self.lib / "current").exists())

    def test_failed_verification_restores_previous_release(self) -> None:
        first = make_package(self.root / "first")
        self.assertEqual(self.install(first).returncode, 0)
        previous = self.link_targets()
        # The host fails only when invoked through the link directory, which
        # the pre-switch check (run from the package directory) does not do.
        second = make_package(
            self.root / "second",
            host_body='case "$0" in *bin-links*) exit 3 ;; esac\necho usage',
        )
        result = self.install(second)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("restoring previous current link", result.stderr)
        self.assertEqual(self.link_targets(), previous)

    @staticmethod
    def package_sha(package: Path) -> str:
        import hashlib

        digest = hashlib.sha256()
        for name in ("codex", "codex-code-mode-host"):
            digest.update((package / "bin" / name).read_bytes())
        return digest.hexdigest()[:12]


if __name__ == "__main__":
    unittest.main()
