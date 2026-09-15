import type { Policy } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { createPolicy } from "./actions";

const STANDARD_LABELS = [
  "PII",
  "DIRECT_IDENTIFIER",
  "INDIRECT_IDENTIFIER",
  "SENSITIVE",
  "FINANCIAL",
  "HEALTH",
  "LOCATION",
  "BEHAVIORAL",
  "COMMERCIAL",
  "CHILDREN_DATA",
  "AUTHENTICATION",
  "INTERNAL",
  "CONFIDENTIAL",
  "IDENTITY",
];

const MARKETING_ACTIONS = [
  "EMAIL_MARKETING",
  "SMS_MARKETING",
  "PUSH_MARKETING",
  "WHATSAPP_MARKETING",
  "PERSONALIZATION",
  "ANALYTICS",
  "ADVERTISING",
  "CROSS_SITE_TARGETING",
  "THIRD_PARTY_EXPORT",
  "DATA_ENRICHMENT",
  "AI_PROCESSING",
];

export default async function GovernancePage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { tenant_id: tenantId, error, note } = await searchParams;

  const policies = tenantId
    ? await apiFetch<Policy[]>(`/policies?tenant_id=${encodeURIComponent(tenantId)}`)
    : null;

  return (
    <main>
      <section>
        <h2>Governance</h2>
        <p className="muted">
          Label-based deny-list policies (Section 15) — default allow, an explicit{" "}
          <span className="mono">deny</span> policy blocks an action for any field carrying that
          label. Standard label vocabulary (Section 13) is documented, not enforced — you can use
          any string. See <a href="/policy-simulator">the policy simulator</a> to check what an
          activation would actually do.
        </p>
        <form method="GET">
          <label>
            Tenant ID
            <input name="tenant_id" defaultValue={tenantId ?? ""} required />
          </label>
          <button type="submit">Load</button>
        </form>
      </section>

      {error && <p className="error">{error}</p>}
      {note && <p className="note">{note}</p>}

      {tenantId && (
        <>
          <section>
            <h2>Create a policy</h2>
            <form action={createPolicy}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Label
                <input name="label" list="standard-labels" required />
                <datalist id="standard-labels">
                  {STANDARD_LABELS.map((l) => (
                    <option key={l} value={l} />
                  ))}
                </datalist>
              </label>
              <label>
                Action
                <select name="action" required>
                  {MARKETING_ACTIONS.map((a) => (
                    <option key={a} value={a}>
                      {a}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Effect
                <select name="effect" defaultValue="deny">
                  <option value="deny">deny</option>
                  <option value="allow">allow</option>
                </select>
              </label>
              <label>
                Priority
                <input name="priority" type="number" defaultValue={0} />
              </label>
              <button type="submit">Create</button>
            </form>
          </section>

          <section>
            <h2>Existing policies</h2>
            {!policies || policies.length === 0 ? (
              <p className="muted">No policies yet for this tenant.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Effect</th>
                    <th>Label</th>
                    <th>Action</th>
                    <th>Priority</th>
                  </tr>
                </thead>
                <tbody>
                  {policies.map((p) => (
                    <tr key={p.id}>
                      <td className={p.effect === "deny" ? "error" : ""}>{p.effect}</td>
                      <td className="mono">{p.label}</td>
                      <td className="mono">{p.action}</td>
                      <td>{p.priority}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        </>
      )}
    </main>
  );
}
