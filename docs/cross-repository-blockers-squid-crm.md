# Cross-Repository Blockers — PyCustomerHub (MessageBirds) ↔ Squid-CRM

Detailed specification of the PyCustomerHub → Squid-CRM integration blocker,
from the PyCustomerHub (MessageBirds) perspective. The shared context is
documented in `Mullassery/Squid-CRM/docs/cross-repository-blockers.md`; this
file records the PyCustomerHub side of the contract and what must change here
to unblock it.

Reference pin: `twentyhq/twenty@08a0e5f1ee2e7db7c1e6ef3aeec9bcef731f4e98`
(Twenty server `2.50.0`).

## 1. The blocker

**Status:** BLOCKED — No shared event-schema contract between PyCustomerHub
and Squid-CRM. Squid's outbound webhook envelope v1 is not the PyCustomerHub
/OCDS envelope: it lacks `event_id`, `event_type`, `timestamp`, `source`,
`identity`, `schema`, `tenant_id`, `correlation_id`. PyCustomerHub `POST /events`
requires a structurally valid OCDS envelope and then a registered event schema,
so Squid webhooks cannot be sent to it unchanged. No batch event endpoint. Edge
ingestion gateway is not deployed and has no write-key auth.

## 2. The two sides

| Aspect | PyCustomerHub (CDP, inbound) | Squid-CRM (CRM, outbound) |
|--------|-----------------------------|---------------------------|
| Transport | `POST /events` (HTTP) → Kafka `events.raw` topic → immutable `events` table | Webhook outbound (HTTP POST to registered `targetUrl`) |
| Envelope | OCDS `EventEnvelope` | Squid envelope v1 |
| Required fields | `event_id`, `event_type`, `timestamp`, `source{type,name}`, `identity[]`, `data`, `schema{name,version}`, `tenant_id` | `id`, `type`, `schemaVersion`, `workspaceId`, `occurredAt`, `data` |
| Identity | top-level `identity[]` array `{namespace, value, primary, source, confidence}` | embedded inside `data` (CRM record fields) |
| Tenancy | `tenant_id` (UUID) | `workspaceId` (UUID) |
| Correlation | `correlation_id` | `x-correlation-id` header + envelope `id` |
| Schema registration | required — event schema must be registered via `POST /schemas` before `POST /events` accepts it | none (envelope is fixed v1) |
| Batch | no batch event endpoint | single event per webhook delivery |
| Auth | edge ingestion gateway write-key auth (not deployed) | HMAC-SHA256 signature (`x-squid-signature`) |

## 3. Evidence

**PyCustomerHub (MessageBirds):**
- OCDS envelope documented in `docs/OCDS.md`: `event_id`, `event_type`,
  `timestamp`, `source{type,name}`, `identity[]`, `context.profile_updates`,
  `data`, `schema{name,version}`, `tenant_id`, `consent`, `governance`,
  `correlation_id`, `trace_id`.
- `POST /events` (`crates/api/src/routes/events.rs`) publishes to Kafka and
  returns `202 ACCEPTED` immediately; deep schema/mixin validation happens
  asynchronously in the worker. The endpoint only checks the envelope is
  structurally well-formed (`envelope.validate_shape()`).
- Event `data` is validated against the event's registered schema. Schemas are
  versioned via `GET/POST /schemas` and `GET /schemas/{name}/{version}`.
- Mixins (`core/identity@1.0`, `core/person@1.0`, `core/contact@1.0`, ...)
  shape customer profiles; `context.profile_updates` contributes field values
  to mixins on the resolved profile.
- Identity graph: every `identity[]` claim resolves against a graph of
  `(tenant, namespace, value) -> profile_id` links.
- Edge ingestion gateway (`edge/ingest-gateway/`) exists but is not deployed;
  no write-key auth yet.

**Squid-CRM:**
- Envelope v1, HMAC signatures, retries, durable `core.webhook_deliveries` log,
  admin replay are integration-tested.
- Python adapter `squid_crm.adapters.to_pycustomerhub_event` maps documented
  top-level OCDS fields but is **unverified** against a live PyCustomerHub
  stack.
- No batch event endpoint on either side; Squid delivers one event per webhook.

## 4. What must be agreed (PyCustomerHub side)

1. **Envelope choice.** Either accept Squid envelope v1 as a valid OCDS event
   (with a defaulted schema), or define a documented field-by-field mapping
   from Squid envelope v1 → OCDS `EventEnvelope`.
2. **Event type + schema registration.** Squid's `type` (e.g. `person.created`)
   must map to a registered PyCustomerHub event schema name/version. Who
   registers schemas and when must be decided.
3. **Identity semantics.** Squid CRM records carry identity inside `data`
   (email, phone, external IDs). OCDS requires a top-level `identity[]` array.
   The mapping must decide which CRM fields become `identity` entries and what
   namespace each uses.
4. **Tenancy.** Squid `workspaceId` → PyCustomerHub `tenant_id`. The
   relationship (1:1? 1:many? workspace-scoped tenant?) must be documented.
5. **Correlation.** Squid's `x-correlation-id` + envelope `id` → OCDS
   `correlation_id`.
6. **Auth.** PyCustomerHub edge ingestion gateway needs write-key auth before
   Squid webhooks can be sent to it unchanged.
7. **Batch.** PyCustomerHub has no batch event endpoint; Squid delivers one
   event per webhook. A batching/buffering decision is needed for high-volume
   workspaces.

## 5. Owner of next move

**PyCustomerHub** must confirm its canonical event schema (or accept Squid
envelope v1) and deploy the edge ingestion gateway with write-key auth. Squid
cannot unilaterally resolve this.

## 6. Related blockers

- **PyReverseETL + Squid:** activation payload + idempotency header — gates
  warehouse→CRM activation (independent of the CDP contract).
- **PyTagShield + Squid:** consent decision contract — PyCustomerHub events
  carry a `consent` field; alignment with Squid's consent model may be needed.
- **ClusterAudienceKit / PyTagManager + Squid:** custom-object schemas —
  PyTagManager → PyCustomerHub ingestion also needs an explicit mapping
  (separate concern from Squid-CRM).
