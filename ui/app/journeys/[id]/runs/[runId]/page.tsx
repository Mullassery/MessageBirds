import type { JourneyRunEvent } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";

export default async function JourneyRunEventsPage({
  params,
}: {
  params: Promise<{ id: string; runId: string }>;
}) {
  const { id, runId } = await params;

  const events = await apiFetch<JourneyRunEvent[]>(`/journey-runs/${runId}/events`);

  return (
    <main>
      <section>
        <h2>Run events</h2>
        <p className="muted">
          <a href={`/journeys/${id}`}>back to journey</a> · run <span className="mono">{runId}</span>
        </p>
      </section>

      <section>
        {!events || events.length === 0 ? (
          <p className="muted">No events recorded yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Node</th>
                <th>Kind</th>
                <th>Detail</th>
                <th>When</th>
              </tr>
            </thead>
            <tbody>
              {events.map((e) => (
                <tr key={e.id}>
                  <td className="mono">{e.node_id}</td>
                  <td>{e.kind}</td>
                  <td className="mono">{e.detail ?? "—"}</td>
                  <td className="muted">{e.created_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
