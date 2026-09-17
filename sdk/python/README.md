# messagebirds

Python client SDK for [MessageBirds](https://github.com/Mullassery/MessageBirds), an
open-source, event-driven Customer Data & Engagement Platform.

Mirrors `sdk/js` exactly in scope: `MessageBirdsClient` covers the core
send-an-event / read-a-profile loop (the same three methods the TS client
exposes), not one method per API endpoint. Everything else — audiences,
governance, channels, templates, journeys — is typed in `messagebirds.types`
for use with your own `requests` calls against the REST API, the same role
`apiFetch<T>` plays against the TS types for `ui/`.

## Install

```bash
pip install messagebirds
```

## Usage

```python
from messagebirds import MessageBirdsClient, EventEnvelope

client = MessageBirdsClient("http://localhost:8080")

envelope: EventEnvelope = {
    "event_id": "...",
    "event_type": "commerce.product_view",
    "timestamp": "2026-09-18T00:00:00Z",
    "source": {"type": "web", "name": "website"},
    "identity": [{"namespace": "anonymous_id", "value": "anon-1", "primary": True, "source": "sdk"}],
    "schema": {"name": "commerce.product_view", "version": "1.0"},
    "data": {"product_id": "p1"},
    "context": {"profile_updates": {"core/person@1.0": {"first_name": "Jane"}}},
    "tenant_id": "...",
}

client.send_event(envelope)
profile = client.find_profile_by_identity("<tenant_id>", "anonymous_id", "anon-1")
```

## Development

```bash
pip install -e ".[dev]"
pytest
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).
