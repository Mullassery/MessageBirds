import type { PolicySimulateResponse } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";

export default async function PolicySimulatorPage({
  searchParams,
}: {
  searchParams: Promise<{
    tenant_id?: string;
    audience_id?: string;
    destination_id?: string;
    action?: string;
  }>;
}) {
  const { tenant_id: tenantId, audience_id: audienceId, destination_id: destinationId, action } =
    await searchParams;

  const ready = tenantId && audienceId && destinationId && action;

  let result: PolicySimulateResponse | null = null;
  let error: string | null = null;
  if (ready) {
    try {
      result = await apiFetch<PolicySimulateResponse>("/policy-simulate", {
        method: "POST",
        body: JSON.stringify({
          tenant_id: tenantId,
          audience_id: audienceId,
          destination_id: destinationId,
          action,
        }),
      });
    } catch (err) {
      error = err instanceof Error ? err.message : "simulation failed";
    }
  }

  return (
    <main>
      <section>
        <h2>Policy simulator</h2>
        <p className="muted">
          &quot;Can this audience be activated to this destination for this action?&quot;
          (Section 49) — checked with the exact same logic real activation uses, so the answer
          matches what would actually happen.
        </p>
        <form method="GET">
          <label>
            Tenant ID
            <input name="tenant_id" defaultValue={tenantId ?? ""} required />
          </label>
          <label>
            Audience ID
            <input name="audience_id" defaultValue={audienceId ?? ""} required />
          </label>
          <label>
            Destination ID
            <input name="destination_id" defaultValue={destinationId ?? ""} required />
          </label>
          <label>
            Marketing action
            <input name="action" defaultValue={action ?? ""} placeholder="ADVERTISING" required />
          </label>
          <button type="submit">Simulate</button>
        </form>
      </section>

      {error && <p className="error">{error}</p>}

      {result && (
        <section>
          <h2>
            Result: <span className={result.final_decision === "allow" ? "" : "error"}>
              {result.final_decision.toUpperCase()}
            </span>
          </h2>

          <h3>Applicable policies / reasons</h3>
          {result.applicable_policies.length === 0 ? (
            <p className="muted">Nothing blocks this.</p>
          ) : (
            <pre>{JSON.stringify(result.applicable_policies, null, 2)}</pre>
          )}

          {result.consent_summary && (
            <>
              <h3>Consent</h3>
              <p>
                {result.consent_summary.granted} of {result.consent_summary.total} current members
                have granted consent for this action&apos;s purpose ({result.consent_summary.missing}{" "}
                missing).
              </p>
            </>
          )}

          <h3>Allowed fields ({result.allowed_fields.length})</h3>
          <p className="mono">{result.allowed_fields.join(", ") || "—"}</p>

          <h3>Blocked fields ({result.blocked_fields.length})</h3>
          <p className="mono error">{result.blocked_fields.join(", ") || "—"}</p>
        </section>
      )}
    </main>
  );
}
