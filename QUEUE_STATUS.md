# Queue status — 2026-10-19

This file records what was shipped, what is blocked/failed, and what is rejected for this repository. It complements `AUDIT_FINDINGS.md`.

## Shipped
- HubSpot and Klaviyo destinations.
- Per-tenant API keys, read-only/writer/admin roles, and forward-auth SSO.

## Blocked / failed
- Live HubSpot/Klaviyo sandbox verification (no vendor credentials).
- Android SDK build verification (no Android toolchain).
- More destination connectors.

## Rejected / deferred
- AI decisioning/agents and audience sequence conditions until the connector layer is broader.

## Policy

Any future feature or upgrade must check `AUDIT_FINDINGS.md` and this file before implementation. A blocked item is retried only when its external dependency is available; a rejected item needs new user evidence or a design change to reopen.
