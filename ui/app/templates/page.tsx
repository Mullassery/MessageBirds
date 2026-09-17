import type { MessageTemplate } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { createTemplate } from "./actions";

export default async function TemplatesPage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { tenant_id: tenantId, error, note } = await searchParams;

  const templates = tenantId
    ? await apiFetch<MessageTemplate[]>(`/templates?tenant_id=${encodeURIComponent(tenantId)}`)
    : null;

  return (
    <main>
      <section>
        <h2>Message templates</h2>
        <p className="muted">
          Registering a template with a name that already exists creates a new version rather than
          overwriting — journey Action nodes pin to a specific template ID, so past runs keep
          using the version they were authored against. Use{" "}
          <span className="mono">{"{{core/person@1.0.first_name}}"}</span> to reference a
          profile&apos;s mixin fields.
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
            <h2>Create a template</h2>
            <form action={createTemplate}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Channel kind
                <input name="channel_kind" defaultValue="webhook" required />
              </label>
              <label>
                Name
                <input name="name" required />
              </label>
              <label>
                Subject (optional)
                <input name="subject" />
              </label>
              <label>
                Body
                <textarea name="body" rows={4} required />
              </label>
              <button type="submit">Create</button>
            </form>
          </section>

          <section>
            <h2>Existing templates</h2>
            {!templates || templates.length === 0 ? (
              <p className="muted">No templates registered for this tenant.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>Version</th>
                    <th>Channel</th>
                    <th>Subject</th>
                    <th>Body</th>
                    <th>ID</th>
                  </tr>
                </thead>
                <tbody>
                  {templates.map((t) => (
                    <tr key={t.id}>
                      <td>{t.name}</td>
                      <td>v{t.version}</td>
                      <td className="mono">{t.channel_kind}</td>
                      <td className="mono">{t.subject ?? "—"}</td>
                      <td className="mono">{t.body}</td>
                      <td className="mono muted">{t.id}</td>
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
