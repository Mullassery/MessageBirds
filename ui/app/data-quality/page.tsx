import type { DeadLetterEventView } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";

export default async function DataQualityPage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string }>;
}) {
  const { tenant_id: tenantId } = await searchParams;

  const events = tenantId
    ? await apiFetch<DeadLetterEventView[]>(`/dead-letter-events?tenant_id=${encodeURIComponent(tenantId)}`)
    : null;

  return (
    <main>
      <section>
        <h2>Data quality</h2>
        <p className="muted">
          Events that failed schema validation and were rejected rather than silently dropped or
          corrupted (see the worker&apos;s reject path).
        </p>
        <form method="GET">
          <label>
            Tenant ID
            <input name="tenant_id" defaultValue={tenantId ?? ""} required />
          </label>
          <button type="submit">Load</button>
        </form>
      </section>

      {tenantId && (
        <section>
          <h2>Dead-lettered events</h2>
          {!events || events.length === 0 ? (
            <p className="muted">No dead-lettered events for this tenant.</p>
          ) : (
            <table>
              <thead>
                <tr>
                  <th>Schema</th>
                  <th>Source</th>
                  <th>Reason</th>
                  <th>Payload</th>
                  <th>When</th>
                </tr>
              </thead>
              <tbody>
                {events.map((e) => (
                  <tr key={e.id}>
                    <td className="mono">
                      {e.schema_name}@{e.schema_version}
                    </td>
                    <td>{e.source}</td>
                    <td className="error">{e.reason}</td>
                    <td className="mono">{JSON.stringify(e.raw_payload)}</td>
                    <td className="muted">{e.created_at}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      )}
    </main>
  );
}
