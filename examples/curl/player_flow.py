#!/usr/bin/env python3
"""Bounded public curl example. No secrets in argv, output, URLs or files."""

import json
import os
from datetime import datetime, timezone
from pathlib import Path
import platform
import re
import stat
import subprocess
from urllib.parse import quote as percent_encode, urlencode, urlsplit
from uuid import UUID, uuid4

CAP = 2 * 1024 * 1024
MARKER = b"\nSTREAMIX_CURL_META "
IDENTITY = {}


class FlowError(Exception):
    """Safe marker only; never attach raw curl output or private JSON."""


def quote(value):
    """Escape a curl stdin configuration string without shell interpolation."""
    return json.dumps(value, ensure_ascii=False)


def installation_id(path):
    """Persist only a descriptive random UUID in an explicit private local file."""
    flags = getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    try:
        descriptor = os.open(path, os.O_RDONLY | flags)
    except FileNotFoundError:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | flags, 0o600)
        with os.fdopen(descriptor, "w") as handle:
            identifier = uuid4()
            handle.write(str(identifier) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
            return str(identifier)
    with os.fdopen(descriptor, "r") as handle:
        metadata = os.fstat(handle.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > 64:
            raise FlowError("INVALID_INSTALLATION_STORE")
        identifier = UUID(handle.read(64).strip())
        if identifier.version != 4:
            raise FlowError("INVALID_INSTALLATION_STORE")
        return str(identifier)


def identity_headers():
    """Build stable public labels; do not read hostname or hardware identity."""
    root = Path(__file__).resolve().parents[2]
    version, build = (root / "VERSION").read_text().strip(), (root / "BUILD").read_text().strip()
    if any(re.fullmatch(r"[A-Za-z0-9._-]{1,64}", text) is None for text in [version, build]):
        raise FlowError("INVALID_BUILD_METADATA")
    system, system_version, arch = platform.system(), os.environ["STREAMIX_SYSTEM_VERSION"], platform.machine()
    name = os.environ["STREAMIX_DEVICE_NAME"]
    for text in [system, system_version, arch]:
        if not text or len(text) > 128 or not text.isascii() or any(char in text for char in "();\\\r\n"):
            raise FlowError("INVALID_BUILD_LABEL")
    if not name or len(name.encode()) > 128 or any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in name) or "://" in name or "stx_" in name.lower():
        raise FlowError("INVALID_DEVICE_NAME")
    result = {
        "User-Agent": f"StreamixKitCurl/{version} ({system} {system_version}; {arch}; build {build}; test) StreamixKit/{version}",
        "X-Streamix-Client": "StreamixKitCurl/" + version,
        "X-Streamix-Device-Id": installation_id(os.environ["STREAMIX_DEVICE_ID_FILE"]),
        "X-Streamix-Device-Name": percent_encode(name, safe="-._~"),
        "X-Streamix-Protocol": "SXAPI/1.0",
    }
    language = os.environ.get("STREAMIX_ACCEPT_LANGUAGE", "en")
    if language:
        if len(language) > 256 or any(not (char.isascii() and (char.isalnum() or char in "-,;= .")) for char in language):
            raise FlowError("INVALID_LANGUAGE_LABEL")
        result["Accept-Language"] = language
    return result


def curl(origin, method, path, bearer=None, ticket=None, body=None, video=False, prefetch=False):
    """Run curl with bounded stdout and a fixed HTTPS trusted origin."""
    parsed = urlsplit(path)
    if not path.startswith("/") or path.startswith("//") or parsed.scheme or parsed.netloc or parsed.fragment:
        raise FlowError("INVALID_PATH")
    config = [
        "silent", 'proto = "=' + urlsplit(origin).scheme + '"', 'noproxy = "*"', "max-redirs = 0",
        "connect-timeout = 10", "max-time = 40", "max-filesize = " + str(1_048_576 if video else CAP),
        "url = " + quote(origin + path), "request = " + quote(method),
        'write-out = "\\nSTREAMIX_CURL_META %{http_code} %{size_download}"',
    ]
    labels = dict(IDENTITY)
    labels["traceparent"] = "00-" + uuid4().hex + "-" + uuid4().hex[:16] + "-01"
    if prefetch:
        labels["Sec-Purpose"] = "prefetch"
    for name, value in labels.items():
        config.append("header = " + quote(name + ": " + value))
    for name, value in [("Authorization", "Bearer " + bearer if bearer else None), ("X-SXAPI-Stream-Ticket", ticket)]:
        if value is not None:
            if any(ord(char) < 32 or ord(char) == 127 for char in value):
                raise FlowError("INVALID_HEADER")
            config.append("header = " + quote(name + ": " + value))
    if body is not None:
        config.extend(['header = "Content-Type: application/json"', "data = " + quote(json.dumps(body, ensure_ascii=False, separators=(",", ":")))])
    if video:
        config.extend(['range = "0-1048575"', 'header = "Accept-Encoding: identity"', 'output = "/dev/null"'])
    process = subprocess.Popen(["curl", "--disable", "--config", "-"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        process.stdin.write(("\n".join(config) + "\n").encode())
        process.stdin.close()
        raw = process.stdout.read(CAP + 256)
        if len(raw) >= CAP + 256:
            process.kill()
            raise FlowError("RESPONSE_LIMIT")
        result = process.wait(timeout=45)
        if result != 0 or MARKER not in raw:
            raise FlowError("TRANSPORT_FAILED")
        payload, metadata = raw.rsplit(MARKER, 1)
        code, size = metadata.decode("ascii").strip().split(" ")
        status, downloaded = int(code), int(float(size))
        if status not in ([206] if video else [200, 204]):
            raise FlowError("HTTP_" + str(status))
        if video:
            if not 0 < downloaded <= 1_048_576:
                raise FlowError("RANGE_LIMIT")
            return downloaded
        return json.loads(payload) if status == 200 else None
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        process.stdout.close()


def main():
    """Keep known session cleanup in finally, including rejected grants."""
    global IDENTITY
    IDENTITY = identity_headers()
    mode = "http" if os.environ.get("STREAMIX_ALLOW_LOCAL_HTTP") == "1" else "https"
    address = os.environ["STREAMIX_ORIGIN"].strip()
    parsed = urlsplit(address if "://" in address else mode + "://" + address)
    if parsed.scheme != mode or not parsed.hostname or parsed.username or parsed.password or parsed.path not in ["", "/"] or parsed.query or parsed.fragment:
        raise FlowError("INVALID_ORIGIN")
    host = "[" + parsed.hostname + "]" if ":" in parsed.hostname else parsed.hostname
    port = parsed.port if parsed.port is not None else (8970 if mode == "http" else 443)
    origin = mode + "://" + host + ":" + str(port)
    media = str(UUID(os.environ["STREAMIX_MEDIA_ID"]))
    profile = json.loads(os.environ["STREAMIX_PROFILE_JSON"])
    if not isinstance(profile, dict) or set(profile) != {"containers", "video_codecs", "audio_codecs", "max_bitrate_bps", "allow_unverified"}:
        raise FlowError("EXPLICIT_PROFILE_REQUIRED")
    capabilities = curl(origin, "GET", "/sxapi/v1/capabilities")
    features = capabilities.get("features", {})
    if features.get("playback_ticket_header") is not True or features.get("credentials_in_urls") is not False or features.get("playback") is not True:
        raise FlowError("HEADER_CONTRACT_UNSUPPORTED")
    token, session_id = None, None
    cleanup_failed = False
    try:
        login = curl(origin, "POST", "/sxapi/v1/sessions", body={"username": os.environ["STREAMIX_USERNAME"], "password": os.environ["STREAMIX_PASSWORD"]})
        token = login["access_token"]
        if login.get("token_type") != "Bearer" or not isinstance(token, str) or not token.startswith("stx_sx_") or len(token) > 2048:
            raise FlowError("LOGIN_CONTRACT")
        print("LOGIN_OK")
        libraries = curl(origin, "GET", "/sxapi/v1/libraries", bearer=token)
        library = str(UUID(libraries[0]["id"]))
        curl(origin, "GET", "/sxapi/v1/media?" + urlencode({"library_id": library, "limit": 1}), bearer=token)
        print("BROWSE_OK")
        request = {"media_id": media, "profile": profile, "device_id": "StreamixKit curl example"}
        if os.environ.get("STREAMIX_VERSION_ID"):
            request["version_id"] = str(UUID(os.environ["STREAMIX_VERSION_ID"]))
        prepared = curl(origin, "POST", "/sxapi/v1/playback/sessions", bearer=token, body=request)
        session_id = str(UUID(prepared["session"]["session_id"]))
        grant = prepared["ticket"]
        path = grant["stream_path"]
        allowed_paths = {"/sxapi/v1/playback/sessions/" + session_id + "/stream", "/nimbus-play/v1/sxapi"}
        if path not in allowed_paths or grant["header_name"] != "X-SXAPI-Stream-Ticket" or not isinstance(grant["ticket"], str) or not 0 < len(grant["ticket"]) <= 4096 or prepared["session"]["mode"] != "direct_play":
            raise FlowError("GRANT_CONTRACT")
        deadline = datetime.fromisoformat(grant["expires_at"].replace("Z", "+00:00"))
        session_deadline = datetime.fromisoformat(prepared["session"]["expires_at"].replace("Z", "+00:00"))
        if deadline <= datetime.now(timezone.utc) or deadline > session_deadline:
            raise FlowError("GRANT_EXPIRY")
        print("PREPARE_OK")
        size = curl(origin, "GET", path, ticket=grant["ticket"], video=True)
        print("RANGE_OK bytes=" + str(size))
    finally:
        if session_id is not None:
            try:
                curl(origin, "DELETE", "/sxapi/v1/playback/sessions/" + session_id, bearer=token)
                print("END_OK")
            except Exception:
                cleanup_failed = True
        if token is not None:
            try:
                curl(origin, "DELETE", "/sxapi/v1/sessions/current", bearer=token)
                print("LOGOUT_OK")
            except Exception:
                cleanup_failed = True
        if cleanup_failed:
            raise FlowError("CLEANUP_FAILED")


if __name__ == "__main__":
    try:
        main()
    except FlowError as error:
        raise SystemExit("FLOW_FAILED: " + str(error))
    except Exception:
        raise SystemExit("FLOW_FAILED: invalid input or response")
