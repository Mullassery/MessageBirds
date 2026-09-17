"""
End-to-end proof that the foundation works, from Python:

  POST /events -> Kafka -> worker (schema validate, identity resolve,
  merge policy, profile projection) -> GET /profiles/by-identity

Run against a live stack: `docker compose up -d postgres redpanda`,
`cargo run -p api`, `cargo run -p worker`, then `python examples/smoke_test.py`.
Mirrors `sdk/js/examples/smoke-test.ts`.
"""

from __future__ import annotations

import os
import sys
import time
import uuid
from datetime import datetime, timezone

from messagebirds import EventEnvelope, MessageBirdsClient

base_url = os.environ.get("MESSAGEBIRDS_API_URL", "http://localhost:8080")
client = MessageBirdsClient(base_url)

tenant_id = str(uuid.uuid4())
anonymous_id = f"anon-{uuid.uuid4()}"


def main() -> None:
    print(f"[smoke-test] tenant={tenant_id} anonymous_id={anonymous_id}")

    envelope: EventEnvelope = {
        "event_id": str(uuid.uuid4()),
        "event_type": "commerce.product_view",
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": {"type": "web", "name": "website"},
        "identity": [
            {"namespace": "anonymous_id", "value": anonymous_id, "primary": True, "source": "smoke-test"}
        ],
        "schema": {"name": "commerce.product_view", "version": "1.0"},
        "data": {"product_id": "p1", "category": "shoes", "price": 79.99},
        "context": {
            "profile_updates": {
                "core/person@1.0": {"first_name": "Jane"},
                "core/contact@1.0": {"email": "jane@example.com"},
                "core/device@1.0": {"device_id": "device-1", "platform": "web"},
            }
        },
        "tenant_id": tenant_id,
    }

    result = client.send_event(envelope)
    print(f"[smoke-test] sent event {result['event_id']}, waiting for the worker to project it...")

    deadline = time.time() + 15
    profile = None
    while time.time() < deadline:
        profile = client.find_profile_by_identity(tenant_id, "anonymous_id", anonymous_id)
        if profile:
            break
        time.sleep(0.5)

    if not profile:
        print("[smoke-test] FAILED: timed out waiting for the profile to materialize — is the worker running?")
        sys.exit(1)

    print("[smoke-test] profile:")
    print(profile)

    mixin_keys = set(profile["mixins"].keys())
    expected = {"core/identity@1.0", "core/person@1.0", "core/contact@1.0", "core/device@1.0"}
    missing = expected - mixin_keys
    if missing:
        print(f"[smoke-test] FAILED: profile is missing expected mixins: {missing}")
        sys.exit(1)
    if not profile["provenance"]:
        print("[smoke-test] FAILED: profile has no field provenance — lineage seed did not get written")
        sys.exit(1)

    print(
        f"[smoke-test] OK — profile {profile['id']} composed {len(mixin_keys)} mixins "
        f"with {len(profile['provenance'])} provenance records"
    )


if __name__ == "__main__":
    main()
