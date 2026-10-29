#!/usr/bin/env python3
"""Check repository-local Markdown links; no remote requests or product code."""

from pathlib import Path
import re
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"!?\[[^\]]*\]\(([^\s)]+)(?:\s+[^)]*)?\)")


def main():
    """Reject broken or repository-escaping relative documentation links."""
    failed = []
    for path in ROOT.rglob("*.md"):
        if any(part in {".git", "target"} for part in path.relative_to(ROOT).parts):
            continue
        for match in LINK.finditer(path.read_text()):
            target = urlsplit(match.group(1))
            if target.scheme or target.netloc or not target.path:
                continue
            resolved = (path.parent / unquote(target.path)).resolve()
            if not resolved.is_relative_to(ROOT) or not resolved.exists():
                failed.append(str(path.relative_to(ROOT)))
    if failed:
        raise SystemExit("LINK_CHECK_FAILED: " + ", ".join(sorted(set(failed))))
    print("LINK_CHECK_OK")


if __name__ == "__main__":
    main()
