#!/usr/bin/env python3
"""Verify public spec digests, exact subset and generated operation inventory."""

from contract import verify_checkout

if __name__ == "__main__":
    try:
        verify_checkout()
    except Exception:
        raise SystemExit("SPEC_CHECK_FAILED: source unbound, contract mismatch or drift")
    print("SPEC_CHECK_OK")
