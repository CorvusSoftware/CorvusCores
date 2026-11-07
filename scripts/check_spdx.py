# SPDX-FileCopyrightText: 2026 CorvusSoftware
# SPDX-License-Identifier: AGPL-3.0-only

"""Fail CI when CorvusSoftware-owned code lacks copyright and AGPL headers."""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
CODE_SUFFIXES = {
    ".rs",
    ".py",
    ".toml",
    ".yml",
    ".yaml",
    ".sh",
    ".c",
    ".cc",
    ".cpp",
    ".h",
    ".hpp",
}
COMMENT_SLASH = {".rs", ".c", ".cc", ".cpp", ".h", ".hpp"}
EXTERNAL_DIRS = {"vendor", "third_party", "target"}


def main() -> int:
    tracked = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        capture_output=True,
        check=True,
    ).stdout
    missing = []
    checked = 0

    for raw in tracked.split(b"\0"):
        if not raw:
            continue
        relative = Path(raw.decode("utf-8"))
        if relative.suffix.lower() not in CODE_SUFFIXES:
            continue
        if any(part in EXTERNAL_DIRS for part in relative.parts):
            continue

        prefix = "//" if relative.suffix.lower() in COMMENT_SLASH else "#"
        lines = (ROOT / relative).read_text(encoding="utf-8").splitlines()[:6]
        copyright_notice = any(
            line.startswith(f"{prefix} SPDX-FileCopyrightText: ") for line in lines
        )
        license_notice = f"{prefix} SPDX-License-Identifier: AGPL-3.0-only" in lines
        checked += 1
        if not (copyright_notice and license_notice):
            missing.append(str(relative))

    if missing:
        print("Files missing CorvusSoftware SPDX notices:", file=sys.stderr)
        for path in missing:
            print(f"  {path}", file=sys.stderr)
        return 1

    print(f"SPDX headers verified on {checked} owned code/config files.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
