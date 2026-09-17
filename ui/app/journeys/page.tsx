import type { JourneyDefinition } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { createJourney } from "./actions";

const EXAMPLE_TRIGGER = `{"kind": "audience_entered", "audience_id": "<audience-uuid>"}`;
const EXAMPLE_NODES = `[
  {"id": "wait1", "kind": "wait", "duration_seconds": 300, "next": "send1"},
  {"id": "send1", "kind": "action", "channel_id": "<channel-uuid>", "template_id": "<template-uuid>", "next": "split1"},
  {"id": "split1", "kind": "split", "branches": [{"next": "end", "weight": 50}, {"next": "end", "weight": 50}]},
  {"id": "end", "kind": "end"}
]`;

export default async function JourneysPage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { tenant_id: tenantId, error, note } = await searchParams;

  const journeys = tenantId
    ? await apiFetch<JourneyDefinition[]>(`/journeys?tenant_id=${encodeURIComponent(tenantId)}`)
    : null;

  return (
    <main>
      <section>
        <h2>Journeys</h2>
        <p className="muted">
          Authored as a flat node list referencing each other by <span className="mono">id</span>{" "}
          (not a nested tree) — no visual canvas yet, this JSON is the source of truth. A journey
          starts automatically for a profile when it enters the trigger audience, or manually via{" "}
          <span className="mono">start</span> on the journey page.
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
            <h2>Create a journey</h2>
            <form action={createJourney}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Name
                <input name="name" required />
              </label>
              <label>
                Entry node id
                <input name="entry_node" defaultValue="wait1" required />
              </label>
              <label>
                Trigger (JSON)
                <textarea name="trigger" rows={2} defaultValue={EXAMPLE_TRIGGER} required />
              </label>
              <label>
                Nodes (JSON array)
                <textarea name="nodes" rows={8} defaultValue={EXAMPLE_NODES} required />
              </label>
              <button type="submit">Create</button>
            </form>
          </section>

          <section>
            <h2>Existing journeys</h2>
            {!journeys || journeys.length === 0 ? (
              <p className="muted">No journeys registered for this tenant.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>Version</th>
                    <th>Status</th>
                    <th>Entry node</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {journeys.map((j) => (
                    <tr key={j.id}>
                      <td>{j.name}</td>
                      <td>v{j.version}</td>
                      <td>{j.status}</td>
                      <td className="mono">{j.entry_node}</td>
                      <td>
                        <a href={`/journeys/${j.id}?tenant_id=${tenantId}`}>view</a>
                      </td>
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
