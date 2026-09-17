import type { Channel } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { createChannel } from "./actions";

export default async function ChannelsPage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { tenant_id: tenantId, error, note } = await searchParams;

  const channels = tenantId
    ? await apiFetch<Channel[]>(`/channels?tenant_id=${encodeURIComponent(tenantId)}`)
    : null;

  return (
    <main>
      <section>
        <h2>Channels</h2>
        <p className="muted">
          Where a journey&apos;s Action nodes send messages. Only{" "}
          <span className="mono">webhook</span> has a real transport — other kinds can be
          registered (e.g. for future SMS/email adapters) but sending against them will fail
          rather than silently succeed.
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
            <h2>Register a channel</h2>
            <form action={createChannel}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Kind
                <input name="kind" defaultValue="webhook" required />
              </label>
              <label>
                Name
                <input name="name" required />
              </label>
              <label>
                Webhook URL
                <input name="webhook_url" type="url" required />
              </label>
              <button type="submit">Register</button>
            </form>
          </section>

          <section>
            <h2>Existing channels</h2>
            {!channels || channels.length === 0 ? (
              <p className="muted">No channels registered for this tenant.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>Kind</th>
                    <th>Config</th>
                    <th>Created</th>
                    <th>ID</th>
                  </tr>
                </thead>
                <tbody>
                  {channels.map((c) => (
                    <tr key={c.id}>
                      <td>{c.name}</td>
                      <td className="mono">{c.kind}</td>
                      <td className="mono">{JSON.stringify(c.config)}</td>
                      <td className="muted">{c.created_at}</td>
                      <td className="mono muted">{c.id}</td>
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
