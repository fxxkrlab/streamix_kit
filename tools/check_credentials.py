#!/usr/bin/env python3
"""Reject credential URLs and accidental private artifacts without printing data."""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
EXTENSIONS = {".md", ".json", ".rs", ".toml", ".sh", ".yml", ".yaml", ".py", ".txt"}
QUERY_SECRET = re.compile(r"[?&](?:ticket|token|api_key|access_token|refresh_token|x-emby-token|authorization|credential)\s*=", re.I)
PATH_SECRET = re.compile(r"/(?:stx_sxplay_|stx_sxsub_|stx_sxrt_|stx_sx_)[A-Za-z0-9_-]+", re.I)
PRIVATE_PATH_SECRET = re.compile(r"/nimbus-play/v1/sxapi(?:-subtitle)?/[0-9a-f]{64}", re.I)
REAL_TOKEN = re.compile(r"\b(?:stx_sxplay_|stx_sxsub_|stx_sxrt_|stx_sx_)[A-Za-z0-9_-]{32,}\b")
PRIVATE_DATA = re.compile(r"(?<![A-Za-z0-9])(?:/Users/[^\s/]+/|/root/|/opt/github-runners/)|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----")
PRODUCTION_ORIGIN = re.compile(r"https?://[^\s/]*halotv\.[^\s/]+", re.I)
IP_LITERAL = re.compile(r"https?://(?:\d{1,3}\.){3}\d{1,3}(?=[:/\s])")
METADATA_HEADER = re.compile(r"^\s*(?:User-Agent|X-Streamix-(?:Client|Device-Id|Device-Name|Protocol)|traceparent)\s*:", re.I)
METADATA_SECRET = re.compile(r"(?:https?://|Bearer\s|stx_|\b(?:username|password|access_token|refresh_token|api_key)\b|(?:\d{1,3}\.){3}\d{1,3})", re.I)


def candidates():
    """Choose authored text, excluding Git, build output and this rule source."""
    for path in ROOT.rglob("*"):
        relative = path.relative_to(ROOT)
        if any(part in {".git", "target", "__pycache__", "artifacts"} for part in relative.parts):
            continue
        if path.is_file() and not path.is_symlink() and path.suffix in EXTENSIONS and path.name != "check_credentials.py":
            yield path


def main():
    """Fail with a file/line location only; never echo the matched credential."""
    failures = []
    for path in candidates():
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if any(pattern.search(line) for pattern in [QUERY_SECRET, PATH_SECRET, PRIVATE_PATH_SECRET, REAL_TOKEN, PRIVATE_DATA, PRODUCTION_ORIGIN, IP_LITERAL]):
                failures.append(f"{path.relative_to(ROOT)}:{number}: prohibited credential URL or private data")
            elif METADATA_HEADER.search(line) and METADATA_SECRET.search(line):
                failures.append(f"{path.relative_to(ROOT)}:{number}: private data in a descriptive header")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        raise SystemExit(1)
    print("CREDENTIAL_CHECK_OK")


if __name__ == "__main__":
    main()
