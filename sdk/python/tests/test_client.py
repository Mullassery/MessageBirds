"""
Exercises `PyCustomerHubClient` against a real local HTTP server (not a
mocked `requests` session) — the same "real transport, verified against a
real listener" discipline used for the webhook adapters in `crates/channels`
and `crates/connectors`.
"""

from __future__ import annotations

import json
import threading
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest

from pycustomerhub import ApiError, PyCustomerHubClient

PROFILE = {
    "id": "profile-1",
    "tenant_id": "tenant-1",
    "mixins": {"core/person@1.0": {"first_name": "Jane"}},
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z",
    "provenance": [],
}


class FakeApiHandler(BaseHTTPRequestHandler):
    def _write(self, status: int, body: dict | None) -> None:
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.end_headers()
        if body is not None:
            self.wfile.write(json.dumps(body).encode())

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        json.loads(self.rfile.read(length)) if length else None
        if self.path == "/events":
            self._write(200, {"event_id": "evt-1"})
        else:
            self._write(500, {"error": "unexpected path"})

    def do_GET(self):
        if self.path == "/profiles/profile-1":
            self._write(200, PROFILE)
        elif self.path == "/profiles/missing":
            self._write(404, {"error": "not found"})
        elif self.path.startswith("/profiles/by-identity"):
            if "value=anon-1" in self.path:
                self._write(200, PROFILE)
            else:
                self._write(404, {"error": "not found"})
        elif self.path == "/profiles/boom":
            self._write(500, {"error": "boom"})
        else:
            self._write(404, {"error": "not found"})

    def log_message(self, *args):
        pass


@pytest.fixture
def client() -> Iterator[PyCustomerHubClient]:
    server = HTTPServer(("127.0.0.1", 0), FakeApiHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield PyCustomerHubClient(f"http://127.0.0.1:{server.server_address[1]}")
    finally:
        server.shutdown()


def test_send_event(client: PyCustomerHubClient) -> None:
    result = client.send_event(
        {
            "event_id": "evt-1",
            "event_type": "commerce.product_view",
            "timestamp": "2026-01-01T00:00:00Z",
            "source": {"type": "web", "name": "website"},
            "identity": [],
            "schema": {"name": "commerce.product_view", "version": "1.0"},
            "tenant_id": "tenant-1",
        }
    )
    assert result == {"event_id": "evt-1"}


def test_get_profile_found(client: PyCustomerHubClient) -> None:
    profile = client.get_profile("profile-1")
    assert profile is not None
    assert profile["id"] == "profile-1"


def test_get_profile_missing_returns_none(client: PyCustomerHubClient) -> None:
    assert client.get_profile("missing") is None


def test_get_profile_error_raises_api_error(client: PyCustomerHubClient) -> None:
    with pytest.raises(ApiError) as exc_info:
        client.get_profile("boom")
    assert exc_info.value.status == 500


def test_find_profile_by_identity_found(client: PyCustomerHubClient) -> None:
    profile = client.find_profile_by_identity("tenant-1", "anonymous_id", "anon-1")
    assert profile is not None
    assert profile["mixins"]["core/person@1.0"]["first_name"] == "Jane"


def test_find_profile_by_identity_missing_returns_none(client: PyCustomerHubClient) -> None:
    assert client.find_profile_by_identity("tenant-1", "anonymous_id", "unknown") is None


def test_deprecated_messagebirds_import_reexports_client() -> None:
    with pytest.warns(DeprecationWarning):
        import messagebirds

    from messagebirds.client import MessageBirdsClient
    from messagebirds.types import EventEnvelope

    assert messagebirds.PyCustomerHubClient is PyCustomerHubClient
    assert messagebirds.MessageBirdsClient is PyCustomerHubClient
    assert MessageBirdsClient is PyCustomerHubClient
    assert EventEnvelope is not None
