#!/usr/bin/env python3
"""Synchronize an explicitly reviewed full/client OpenAPI pair; no credentials."""

import argparse
from datetime import datetime, timezone
import os
from pathlib import Path
import re
import tempfile
import urllib.parse
import urllib.request

from contract import ROOT, FULL_NAME, CLIENT_NAME, encoded, inventory, read_document, require, sha256, validate_pair


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """A sync must never follow an unreviewed download origin."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("spec redirect rejected")


def obtain(location):
    """Read one bounded local reviewed export or unauthenticated HTTPS URL."""
    if location.startswith("https://"):
        parsed = urllib.parse.urlsplit(location)
        require(not parsed.username and not parsed.password and not parsed.query and not parsed.fragment, "credential-free spec URL required")
        require(bool(parsed.hostname), "spec host required")
        opener = urllib.request.build_opener(NoRedirect(), urllib.request.ProxyHandler({}))
        request = urllib.request.Request(location, headers={"Accept": "application/json"})
        with opener.open(request, timeout=30) as response:
            require(response.status == 200, "spec HTTP status")
            raw = response.read(16 * 1024 * 1024 + 1)
    else:
        require("://" not in location, "only HTTPS or local exports supported")
        path = Path(location)
        require(path.is_file() and not path.is_symlink(), "regular spec export required")
        raw = path.read_bytes()
    require(len(raw) <= 16 * 1024 * 1024, "spec byte limit")
    return raw


def replace(path, raw):
    """Replace a generated file atomically after the full pair has validated."""
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as handle:
        temporary = Path(handle.name)
        try:
            handle.write(raw)
            handle.flush()
            os.fsync(handle.fileno())
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)


def main():
    """Synchronize only when exact expected hashes and provenance are provided."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--full", required=True, help="reviewed public full export or HTTPS URL")
    parser.add_argument("--client", required=True, help="reviewed server-generated client export or HTTPS URL")
    parser.add_argument("--full-sha256", required=True)
    parser.add_argument("--client-sha256", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--version", required=True, help="intended server SemVer; not an assertion of publication")
    parser.add_argument("--release", help="actual published release tag, only when one exists")
    parser.add_argument("--minimum-server-version", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.commit) is not None, "full upstream commit required")
    for value in [args.version, args.minimum_server_version] + ([args.release] if args.release else []):
        require(re.fullmatch(r"v?\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?", value) is not None, "release version required")
    for value in [args.full_sha256, args.client_sha256]:
        require(re.fullmatch(r"[0-9a-f]{64}", value) is not None, "exact expected digest required")
    full_raw, client_raw = obtain(args.full), obtain(args.client)
    require(sha256(full_raw) == args.full_sha256 and sha256(client_raw) == args.client_sha256, "reviewed export digest mismatch")
    full, client = read_document(full_raw), read_document(client_raw)
    validate_pair(full, client)
    require(full.get("info", {}).get("version") == args.version.removeprefix("v"), "intended version differs from spec")
    source = {
        "schema_version": 1, "status": "BOUND", "upstream_repository": "fxxkrlab/streamix_server",
        "upstream_commit": args.commit, "upstream_release": args.release,
        "intended_server_version": args.version.removeprefix("v"),
        "candidate_status": "RELEASE_REFERENCE_PROVIDED" if args.release else "UNRELEASED",
        "compatibility_status": "NOT_VALIDATED",
        "minimum_server_version": args.minimum_server_version, "protocol_major": 1,
        "required_ticket_transport": "headers", "synchronized_at": datetime.now(timezone.utc).isoformat(),
        "files": {name: {"sha256": sha256(raw), "bytes": len(raw)} for name, raw in [(FULL_NAME, full_raw), (CLIENT_NAME, client_raw)]},
    }
    directory = ROOT / "spec"
    directory.mkdir(exist_ok=True)
    # SOURCE is last: interruption leaves an invalid/old digest, never false BOUND acceptance.
    for name, raw in [(FULL_NAME, full_raw), (CLIENT_NAME, client_raw), ("OPERATIONS.json", encoded(inventory(client))), ("SOURCE.json", encoded(source))]:
        replace(directory / name, raw)
    print("SPEC_SYNCED_REVIEW_REQUIRED")


if __name__ == "__main__":
    try:
        main()
    except Exception:
        raise SystemExit("SPEC_SYNC_FAILED: reviewed provenance, contract or export mismatch")
