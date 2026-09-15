import { notFound } from "next/navigation";
import type { AudienceDefinition, Membership } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";

export default async function AudienceMembersPage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ tenant_id?: string }>;
}) {
  const { id } = await params;
  const { tenant_id: tenantId } = await searchParams;

  if (!tenantId) {
    return (
      <main>
        <p className="error">A tenant_id query parameter is required.</p>
      </main>
    );
  }

  const [audience, members] = await Promise.all([
    apiFetch<AudienceDefinition>(`/audiences/${id}?tenant_id=${encodeURIComponent(tenantId)}`),
    apiFetch<Membership[]>(`/audiences/${id}/members`),
  ]);

  if (!audience) {
    notFound();
  }

  return (
    <main>
      <section>
        <h2>
          {audience.name} <span className="muted">v{audience.version}</span>
        </h2>
        <p className="muted">
          <a href={`/audiences?tenant_id=${tenantId}`}>back to audiences</a>
        </p>
        <pre>{JSON.stringify(audience.conditions, null, 2)}</pre>
      </section>

      <section>
        <h2>Current members</h2>
        {!members || members.length === 0 ? (
          <p className="muted">No profiles currently match this audience.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Profile</th>
                <th>Entered</th>
              </tr>
            </thead>
            <tbody>
              {members.map((m) => (
                <tr key={m.profile_id}>
                  <td className="mono">
                    <a href={`/profiles/${m.profile_id}`}>{m.profile_id}</a>
                  </td>
                  <td className="muted">{m.entered_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
