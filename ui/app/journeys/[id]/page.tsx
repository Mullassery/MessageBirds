import { notFound } from "next/navigation";
import type { JourneyDefinition, JourneyRun } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { startJourney } from "../actions";

export default async function JourneyPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { id } = await params;
  const { tenant_id: tenantId, error, note } = await searchParams;

  if (!tenantId) {
    return (
      <main>
        <p className="error">A tenant_id query parameter is required.</p>
      </main>
    );
  }

  const [journey, runs] = await Promise.all([
    apiFetch<JourneyDefinition>(`/journeys/${id}?tenant_id=${encodeURIComponent(tenantId)}`),
    apiFetch<JourneyRun[]>(`/journeys/${id}/runs`),
  ]);

  if (!journey) {
    notFound();
  }

  return (
    <main>
      <section>
        <h2>
          {journey.name} <span className="muted">v{journey.version}</span>
        </h2>
        <p className="muted">
          <a href={`/journeys?tenant_id=${tenantId}`}>back to journeys</a> · {journey.status} ·
          entry node <span className="mono">{journey.entry_node}</span>
        </p>
        <pre>{JSON.stringify(journey.trigger, null, 2)}</pre>
      </section>

      {error && <p className="error">{error}</p>}
      {note && <p className="note">{note}</p>}

      <section>
        <h2>Nodes</h2>
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Kind</th>
              <th>Detail</th>
            </tr>
          </thead>
          <tbody>
            {journey.nodes.map((n) => (
              <tr key={n.id}>
                <td className="mono">{n.id}</td>
                <td className="mono">{n.kind}</td>
                <td className="mono">{JSON.stringify(n)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Manually enroll a profile</h2>
        <form action={startJourney}>
          <input type="hidden" name="tenant_id" value={tenantId} />
          <input type="hidden" name="journey_id" value={journey.id} />
          <label>
            Profile ID
            <input name="profile_id" required />
          </label>
          <button type="submit">Start run</button>
        </form>
      </section>

      <section>
        <h2>Runs</h2>
        {!runs || runs.length === 0 ? (
          <p className="muted">No runs yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Profile</th>
                <th>Status</th>
                <th>Current node</th>
                <th>Wakes at</th>
                <th>Started</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {runs.map((r) => (
                <tr key={r.id}>
                  <td className="mono">
                    <a href={`/profiles/${r.profile_id}`}>{r.profile_id}</a>
                  </td>
                  <td>{r.status}</td>
                  <td className="mono">{r.current_node}</td>
                  <td className="muted">{r.wake_at ?? "—"}</td>
                  <td className="muted">{r.started_at}</td>
                  <td>
                    <a href={`/journeys/${journey.id}/runs/${r.id}`}>events</a>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
