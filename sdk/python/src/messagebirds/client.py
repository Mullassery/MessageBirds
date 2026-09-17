from __future__ import annotations

import requests

from .types import EventEnvelope, ProfileView


class ApiError(Exception):
    """Mirrors `sdk/js`'s `ApiError` — raised for any non-2xx, non-404 response."""

    def __init__(self, status: int, message: str) -> None:
        super().__init__(message)
        self.status = status


class MessageBirdsClient:
    """
    Mirrors `sdk/js`'s `MessageBirdsClient` exactly: the core send-an-event,
    read-a-profile loop, not a method per API endpoint. For everything
    else, build your own request against `base_url` — the `messagebirds`
    package's `types` module gives you typed dict shapes for the response,
    the same role `apiFetch<T>` plays against the TS types for `ui/`.
    """

    def __init__(self, base_url: str, *, session: requests.Session | None = None) -> None:
        self.base_url = base_url.rstrip("/")
        self._session = session or requests.Session()

    def send_event(self, envelope: EventEnvelope) -> dict[str, str]:
        res = self._session.post(f"{self.base_url}/events", json=envelope)
        if not res.ok:
            raise ApiError(res.status_code, f"POST /events failed: {res.text}")
        return res.json()

    def get_profile(self, profile_id: str) -> ProfileView | None:
        res = self._session.get(f"{self.base_url}/profiles/{profile_id}")
        if res.status_code == 404:
            return None
        if not res.ok:
            raise ApiError(res.status_code, f"GET /profiles/{profile_id} failed: {res.text}")
        return res.json()

    def find_profile_by_identity(
        self, tenant_id: str, namespace: str, value: str
    ) -> ProfileView | None:
        res = self._session.get(
            f"{self.base_url}/profiles/by-identity",
            params={"tenant_id": tenant_id, "namespace": namespace, "value": value},
        )
        if res.status_code == 404:
            return None
        if not res.ok:
            raise ApiError(res.status_code, f"GET /profiles/by-identity failed: {res.text}")
        return res.json()
