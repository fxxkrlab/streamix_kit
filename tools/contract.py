"""Public contract invariants shared by synchronization and server CI."""

import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HTTP_METHODS = {"get", "post", "put", "patch", "delete", "head", "options", "trace"}
PLAYER_ROOTS = {"capabilities", "sessions", "users", "libraries", "media", "playback", "series", "collections", "persons", "account"}
FULL_NAME = "sxapi-v1.openapi.json"
CLIENT_NAME = "sxapi-v1-client.openapi.json"


def require(condition, message):
    """Raise a sanitized invariant failure without document data."""
    if not condition:
        raise ValueError(message)


def sha256(data):
    """Return a content digest for exact synchronized bytes."""
    return hashlib.sha256(data).hexdigest()


def read_document(raw):
    """Decode a bounded OpenAPI 3.1 document with duplicate-key rejection."""
    require(0 < len(raw) <= 16 * 1024 * 1024, "spec byte limit")

    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    doc = json.loads(raw, object_pairs_hook=unique_pairs)
    require(isinstance(doc, dict), "spec object required")
    require(str(doc.get("openapi", "")).startswith("3.1."), "OpenAPI 3.1 required")
    require(doc.get("servers") == [{"url": "/sxapi/v1"}], "unexpected server base")
    require(isinstance(doc.get("paths"), dict) and doc["paths"], "paths required")
    return doc


def walk_refs(value):
    """Yield local component references; remote references are forbidden."""
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "$ref":
                require(isinstance(child, str) and child.startswith("#/components/"), "nonlocal reference")
                yield child
            else:
                yield from walk_refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk_refs(child)


def component(doc, ref):
    """Resolve an exact component JSON pointer."""
    parts = ref.split("/")
    require(len(parts) == 4, "nested component pointer unsupported")
    _, _, group, name = parts
    name = name.replace("~1", "/").replace("~0", "~")
    require(name in doc.get("components", {}).get(group, {}), "unresolved component")
    return group, name, doc["components"][group][name]


def validate_pair(full, client):
    """Require exact operation copies and the complete player reference closure."""
    require(full.get("info", {}).get("version") == client.get("info", {}).get("version"), "spec versions differ")
    for path, item in client["paths"].items():
        require(path.startswith("/") and path.split("/")[1] in PLAYER_ROOTS, "nonplayer path in subset")
        if path.startswith("/users/"):
            require(path == "/users/me" or path.startswith("/users/me/"), "non-current user path")
        if path.startswith("/account/"):
            require(path == "/account/credentials" or path.startswith("/account/credentials/"), "non-credential account path")
        require(path in full["paths"] and item == full["paths"][path], "subset path differs from full")
    # The subset is copied from the server, not independently selected here.
    # Roots below still enforce presence of the documented core flow.
    for path in ["/capabilities", "/sessions", "/libraries", "/media", "/playback/sessions"]:
        require(path in client["paths"], "core player path missing")
    # Reconstruct the server's transitive component/security closure to reject
    # unused administration schemas accidentally included in a client export.
    needed = set(walk_refs(client["paths"]))
    def security_refs(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key == "security":
                    for requirement in child:
                        for name in requirement:
                            yield "#/components/securitySchemes/" + name
                else:
                    yield from security_refs(child)
        elif isinstance(value, list):
            for child in value:
                yield from security_refs(child)
    needed.update(security_refs(client["paths"]))
    needed.update(security_refs({"security": client.get("security", [])}))
    visited = set()
    while needed - visited:
        ref = sorted(needed - visited)[0]
        visited.add(ref)
        _, _, value = component(client, ref)
        needed.update(walk_refs(value))
    actual = {"#/components/" + group + "/" + name.replace("~", "~0").replace("/", "~1")
              for group, values in client.get("components", {}).items() for name in values}
    require(actual == visited, "subset component closure is not exact")
    for ref in visited:
        group, name, value = component(client, ref)
        require(value == full.get("components", {}).get(group, {}).get(name), "subset component differs")
    for doc in [full, client]:
        for ref in walk_refs(doc):
            component(doc, ref)
        for scheme in doc.get("components", {}).get("securitySchemes", {}).values():
            if scheme.get("type") == "apiKey":
                require(scheme.get("in") == "header", "credential security scheme is not header-only")
    features = client.get("components", {}).get("schemas", {}).get("Capabilities", {}).get("properties", {}).get("features", {}).get("properties", {})
    require(features.get("playback_ticket_header", {}).get("const") is True, "header discovery missing")
    require(features.get("credentials_in_urls", {}).get("const") is False, "URL credential prohibition missing")
    ticket = client.get("components", {}).get("schemas", {}).get("StreamTicket", {})
    # v1 adds the header fields without tightening its historical required list.
    # Runtime code requires actual fields when the server advertises the feature.
    require({"stream_path", "ticket", "header_name", "expires_at"} <= set(ticket.get("properties", {})), "video grant header properties missing")
    require(ticket.get("properties", {}).get("header_name", {}).get("const") == "X-SXAPI-Stream-Ticket", "unexpected video header")
    subtitle = client.get("components", {}).get("schemas", {}).get("SubtitleTicket", {})
    require(subtitle.get("properties", {}).get("header_name", {}).get("const") == "X-SXAPI-Subtitle-Ticket", "unexpected subtitle header")


def inventory(client):
    """Generate a deterministic operation index, without private source metadata."""
    return {
        "schema_version": 1,
        "protocol_major": 1,
        "operations": [
            {"method": method.upper(), "path": path, "operation_id": item[method]["operationId"]}
            for path, item in sorted(client["paths"].items())
            for method in sorted(HTTP_METHODS & item.keys())
        ],
        "typed_sdk_schema_sha256": {
            name: sha256(json.dumps(client["components"]["schemas"][name], ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode())
            for name in ["SessionRequest", "Session", "OwnCredentialRefresh", "OwnCredentialRefreshed", "User", "Library", "Media", "MediaPage", "Version", "VersionPage", "PlaybackProfile", "PlaybackRequest", "PlaybackSession", "PreparedPlayback", "StreamTicket", "PlaybackProgress", "ProgressAck", "SubtitleInventory", "SubtitleTicket"]
        },
    }


def encoded(value):
    """Serialize public metadata consistently."""
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def verify_checkout():
    """Verify exact provenance and generated inventory; unbound source fails."""
    source = json.loads((ROOT / "spec/SOURCE.json").read_text())
    require(source.get("status") == "BOUND", "upstream source is not bound")
    require(source.get("upstream_repository") == "fxxkrlab/streamix_server", "unexpected upstream repository")
    require(re.fullmatch(r"[0-9a-f]{40}", source.get("upstream_commit", "")) is not None, "upstream full commit required")
    require(bool(source.get("intended_server_version")) and bool(source.get("minimum_server_version")), "version binding required")
    require(source.get("upstream_release") is not None or source.get("candidate_status") == "UNRELEASED", "unpublished candidate must be explicit")
    docs = []
    for name in [FULL_NAME, CLIENT_NAME]:
        raw = (ROOT / "spec" / name).read_bytes()
        require(source.get("files", {}).get(name, {}).get("sha256") == sha256(raw), "synchronized spec digest mismatch")
        require(source.get("files", {}).get(name, {}).get("bytes") == len(raw), "synchronized spec size mismatch")
        docs.append(read_document(raw))
    validate_pair(*docs)
    require(docs[0]["info"]["version"] == source["intended_server_version"], "source version differs from spec")
    require((ROOT / "spec/OPERATIONS.json").read_bytes() == encoded(inventory(docs[1])), "operation inventory drift")
    return source, docs[1]
