#!/usr/bin/env python3
"""Check the deliberately wrapped Rust wire shapes against the pinned client spec."""

from contract import require, verify_checkout

EXPECTED = {
    "SessionRequest": {"username": "string", "password": "string:password"},
    "Session": {"access_token": "string", "token_type": "const:Bearer", "user": "ref:User"},
    "OwnCredentialRefresh": {"refresh_token": "string"},
    "OwnCredentialRefreshed": {"access_token": "string", "token_type": "const:Bearer"},
    "User": {"id": "string:uuid", "username": "string", "library_ids": "array:string:uuid"},
    "Library": {"id": "string:uuid", "name": "string", "library_type": "string", "library_kind": "string"},
    "Media": {"id": "string:uuid", "name": "string", "media_type": "null|string"},
    "Version": {"id": "string:uuid", "media_id": "string:uuid", "episode_id": "null|string:uuid", "technical_status": "string", "duration_secs": "null|number", "video_codec": "null|string", "audio_codec": "null|string"},
    "MediaPage": {"data": "array:ref:Media", "total": "integer:int64", "next_cursor": "null|string"},
    "VersionPage": {"data": "array:ref:Version", "total": "integer:int64", "next_cursor": "null|string"},
    "PlaybackProfile": {"containers": "array:string", "video_codecs": "array:string", "audio_codecs": "array:string", "max_bitrate_bps": "integer:int64|null", "allow_unverified": "boolean"},
    "PlaybackRequest": {"media_id": "string:uuid", "episode_id": "null|string:uuid", "version_id": "null|string:uuid", "profile": "ref:PlaybackProfile", "device_id": "null|string"},
    "PlaybackSession": {"session_id": "string:uuid", "media_id": "string:uuid", "episode_id": "null|string:uuid", "version_id": "string:uuid", "mode": "const:direct_play", "compatibility": "string", "expires_at": "string:date-time"},
    "PreparedPlayback": {"session": "ref:PlaybackSession", "ticket": "ref:StreamTicket"},
    "StreamTicket": {"stream_path": "string", "ticket": "string", "header_name": "const:X-SXAPI-Stream-Ticket", "expires_at": "string:date-time"},
    "PlaybackProgress": {"sequence": "integer:int64", "position_ms": "integer:int64", "completed": "boolean"},
    "ProgressAck": {"session_id": "string:uuid", "sequence": "integer:int64", "position_ms": "integer:int64", "completed": "boolean", "state_revision": "string", "recorded_at": "string:date-time"},
    "SubtitleInventory": {"media_id": "string:uuid", "episode_id": "null|string:uuid", "version_id": "string:uuid", "revision": "string", "embedded_status": "const:unknown", "data": "array:object"},
    "SubtitleTicket": {"ticket": "string", "header_name": "const:X-SXAPI-Subtitle-Ticket", "read_path": "string", "expires_at": "string:date-time"},
}


def wire_kind(schema):
    """Normalize only the scalar/ref shapes used by this limited typed facade."""
    if "$ref" in schema:
        return "ref:" + schema["$ref"].rsplit("/", 1)[1]
    if "const" in schema:
        return "const:" + str(schema["const"])
    if "anyOf" in schema:
        return "|".join(sorted(wire_kind(item) for item in schema["anyOf"]))
    kind = schema.get("type")
    if isinstance(kind, list):
        return "|".join(sorted(wire_kind({**schema, "type": item}) for item in kind))
    if kind is None and "enum" in schema:
        kind = "string"
    if kind == "array":
        return "array:" + wire_kind(schema["items"])
    return str(kind) + (":" + schema["format"] if "format" in schema else "")


def check(client):
    """Check supported fields; unknown additive response fields stay preserved."""
    schemas = client["components"]["schemas"]
    for name, fields in EXPECTED.items():
        require(name in schemas, "typed SDK schema missing")
        properties = schemas[name].get("properties", {})
        for field, expected in fields.items():
            require(field in properties and wire_kind(properties[field]) == expected, "typed SDK wire shape changed")
    for name in ["SessionRequest", "OwnCredentialRefresh", "PlaybackProfile", "PlaybackRequest", "PlaybackProgress"]:
        require(set(schemas[name].get("properties", {})) == set(EXPECTED[name]), "request fields require explicit SDK review")


if __name__ == "__main__":
    try:
        _, document = verify_checkout()
        check(document)
    except Exception:
        raise SystemExit("SDK_CONTRACT_CHECK_FAILED: review typed wire models against synchronized specification")
    print("SDK_CONTRACT_CHECK_OK")
