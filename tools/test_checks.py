"""Pure public data regression cases, executed only in designated server CI."""

import copy
import unittest

from contract import read_document, validate_pair
from check_credentials import QUERY_SECRET, PATH_SECRET, PRIVATE_PATH_SECRET, REAL_TOKEN, METADATA_HEADER, METADATA_SECRET
from check_sdk_contract import wire_kind


def fixture():
    schemas = {
        "Capabilities": {"properties": {"features": {"properties": {"playback_ticket_header": {"const": True}, "credentials_in_urls": {"const": False}}}}},
        "StreamTicket": {"required": ["stream_url", "expires_at"], "properties": {"stream_path": {"type": "string"}, "ticket": {"type": "string"}, "expires_at": {"type": "string"}, "header_name": {"const": "X-SXAPI-Stream-Ticket"}}},
        "SubtitleTicket": {"properties": {"header_name": {"const": "X-SXAPI-Subtitle-Ticket"}}},
    }
    paths = {name: {"get": {"operationId": name[1:], "responses": {}}} for name in ["/capabilities", "/sessions", "/libraries", "/media", "/playback/sessions"]}
    paths["/capabilities"]["get"]["responses"] = {"200": {"description": "fixture", "content": {"application/json": {"schema": {"oneOf": [{"$ref": "#/components/schemas/" + name} for name in schemas]}}}}}
    return {"openapi": "3.1.0", "info": {"version": "1.0.0"}, "servers": [{"url": "/sxapi/v1"}], "paths": paths, "components": {"schemas": schemas}}


class ContractChecks(unittest.TestCase):
    def test_same_generated_pair(self):
        doc = fixture()
        validate_pair(doc, copy.deepcopy(doc))

    def test_operation_drift_rejected(self):
        doc = fixture()
        client = copy.deepcopy(doc)
        client["paths"]["/media"]["get"]["summary"] = "drift"
        with self.assertRaises(ValueError):
            validate_pair(doc, client)

    def test_old_header_contract_rejected(self):
        doc = fixture()
        del doc["components"]["schemas"]["Capabilities"]["properties"]["features"]["properties"]["playback_ticket_header"]
        with self.assertRaises(ValueError):
            validate_pair(doc, doc)

    def test_duplicate_json_rejected(self):
        with self.assertRaises(ValueError):
            read_document(b'{"openapi":"3.1.0","openapi":"3.1.0"}')

    def test_query_security_rejected(self):
        doc = fixture()
        doc["components"]["securitySchemes"] = {"bad": {"type": "apiKey", "in": "query"}}
        with self.assertRaises(ValueError):
            validate_pair(doc, doc)

    def test_unused_schema_rejected(self):
        doc = fixture()
        doc["components"]["schemas"]["Unneeded"] = {"type": "string"}
        with self.assertRaises(ValueError):
            validate_pair(doc, doc)

    def test_bad_url_credentials_detected_without_secret_literals(self):
        for key in ["ticket", "token", "api_key", "access_token", "refresh_token", "X-Emby-Token"]:
            self.assertIsNotNone(QUERY_SECRET.search("https://streamix.example/read?" + key + "=fixture"))
        self.assertIsNotNone(PATH_SECRET.search("/" + "stx_sxplay_" + "fixture"))
        self.assertIsNotNone(PRIVATE_PATH_SECRET.search("/nimbus-play/v1/sxapi/" + "a" * 64))
        self.assertIsNotNone(REAL_TOKEN.search("stx_sx_" + "a" * 64))
        self.assertIsNone(QUERY_SECRET.search("/media?cursor=opaque&limit=20"))

    def test_header_label_privacy_rule(self):
        bad = "X-Streamix-Device-Name: " + "https://streamix.example"
        self.assertIsNotNone(METADATA_HEADER.search(bad))
        self.assertIsNotNone(METADATA_SECRET.search(bad))
        self.assertIsNone(METADATA_SECRET.search("X-Streamix-Client: ExamplePlayer/1.0.0"))

    def test_wire_kind_preserves_nullable_numeric_and_refs(self):
        self.assertEqual(wire_kind({"anyOf": [{"type": "integer", "format": "int64"}, {"type": "null"}]}), "integer:int64|null")
        self.assertEqual(wire_kind({"$ref": "#/components/schemas/User"}), "ref:User")


if __name__ == "__main__":
    unittest.main()
