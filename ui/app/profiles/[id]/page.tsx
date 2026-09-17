import { notFound } from "next/navigation";
import type {
  ConsentView,
  EventSummary,
  IdentityView,
  JourneyRun,
  Membership,
  MergeSuggestion,
  ProfileView,
} from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { mergeProfile, recordConsent, splitIdentity } from "./actions";

export default async function ProfilePage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ error?: string; note?: string }>;
}) {
  const { id } = await params;
  const { error, note } = await searchParams;

  const profile = await apiFetch<ProfileView>(`/profiles/${id}`);
  if (!profile) {
    notFound();
  }

  const [identity, events, suggestions, audiences, consent, journeyRuns] = await Promise.all([
    apiFetch<IdentityView>(`/profiles/${id}/identity`),
    apiFetch<EventSummary[]>(`/profiles/${id}/events`),
    apiFetch<MergeSuggestion[]>(`/profiles/${id}/merge-suggestions`),
    apiFetch<Membership[]>(`/profiles/${id}/audiences`),
    apiFetch<ConsentView>(`/profiles/${id}/consent?tenant_id=${profile.tenant_id}`),
    apiFetch<JourneyRun[]>(`/profiles/${id}/journeys?tenant_id=${profile.tenant_id}`),
  ]);

  const boundMerge = mergeProfile.bind(null, id);
  const boundSplit = splitIdentity.bind(null, id);
  const boundConsent = recordConsent.bind(null, id);

  return (
    <main>
      {error && <p className="error">{error}</p>}
      {note && <p className="note">{note}</p>}

      <section>
        <h2>Profile</h2>
        <p className="mono muted">{profile.id}</p>
        <p className="muted">
          tenant {profile.tenant_id} · created {profile.created_at} · updated{" "}
          {profile.updated_at}
        </p>
        <pre>{JSON.stringify(profile.mixins, null, 2)}</pre>
      </section>

      <section>
        <h2>Field provenance</h2>
        {profile.provenance.length === 0 ? (
          <p className="muted">No provenance recorded yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Mixin</th>
                <th>Field</th>
                <th>Value</th>
                <th>Source</th>
                <th>Policy</th>
                <th>Updated</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {profile.provenance.map((p) => (
                <tr key={`${p.mixin_key}.${p.field_path}`}>
                  <td className="mono">{p.mixin_key}</td>
                  <td className="mono">{p.field_path}</td>
                  <td>{String(p.value)}</td>
                  <td>{p.source}</td>
                  <td>{p.applied_policy}</td>
                  <td className="muted">{p.updated_at}</td>
                  <td>
                    <a
                      href={`/profiles/${id}/lineage?mixin=${encodeURIComponent(
                        p.mixin_key,
                      )}&field=${encodeURIComponent(p.field_path)}`}
                    >
                      lineage
                    </a>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Identity graph</h2>
        <p className="muted">
          Values are stored hashed and shown that way here — see the event
          timeline below for the plaintext claim a split needs.
        </p>
        <table>
          <thead>
            <tr>
              <th>Namespace</th>
              <th>Value hash</th>
              <th>Confidence</th>
              <th>Source</th>
              <th>First seen</th>
            </tr>
          </thead>
          <tbody>
            {identity?.nodes.map((n) => (
              <tr key={n.id}>
                <td className="mono">{n.namespace}</td>
                <td className="mono">{n.value_hash.slice(0, 16)}…</td>
                <td>{n.confidence}</td>
                <td>{n.source}</td>
                <td className="muted">{n.first_seen_at}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <h3>Merge / split audit trail</h3>
        {(!identity || identity.audit.length === 0) ? (
          <p className="muted">No merges or splits recorded.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Kind</th>
                <th>Claim</th>
                <th>Profile</th>
                <th>Previous profile</th>
                <th>Source</th>
                <th>When</th>
              </tr>
            </thead>
            <tbody>
              {identity.audit.map((a) => (
                <tr key={a.id}>
                  <td>{a.kind}</td>
                  <td className="mono">{a.namespace ?? "—"}</td>
                  <td className="mono">{a.profile_id}</td>
                  <td className="mono">{a.previous_profile_id ?? "—"}</td>
                  <td>{a.source}</td>
                  <td className="muted">{a.created_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <h3>Split off an identity claim</h3>
        <p className="muted">
          Unlinks one claim going forward. Does not migrate any mixin/profile
          data already attributed to this profile — see the note the API
          returns after submitting.
        </p>
        <form action={boundSplit}>
          <label>
            Namespace
            <input name="namespace" required />
          </label>
          <label>
            Value
            <input name="value" required />
          </label>
          <button type="submit">Split</button>
        </form>
      </section>

      <section>
        <h2>Merge suggestions</h2>
        <p className="muted">
          Confidence-scored candidates sharing this profile&apos;s device id —
          never merged automatically.
        </p>
        {!suggestions || suggestions.length === 0 ? (
          <p className="muted">No candidates above the threshold.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Candidate profile</th>
                <th>Score</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {suggestions.map((s) => (
                <tr key={s.profile_id}>
                  <td className="mono">
                    <a href={`/profiles/${s.profile_id}`}>{s.profile_id}</a>
                  </td>
                  <td>{s.score.toFixed(2)}</td>
                  <td>
                    <form action={boundMerge}>
                      <input type="hidden" name="from_profile_id" value={s.profile_id} />
                      <button type="submit">Merge into this profile</button>
                    </form>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Audiences</h2>
        <p className="muted">
          Evaluated in real time as this profile&apos;s mixins change — see{" "}
          <a href="/audiences">the audience list</a> to create more.
        </p>
        {!audiences || audiences.length === 0 ? (
          <p className="muted">Not a member of any audience.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Audience</th>
                <th>Entered</th>
                <th>Exited</th>
              </tr>
            </thead>
            <tbody>
              {audiences.map((m) => (
                <tr key={m.audience_id}>
                  <td className="mono">
                    <a href={`/audiences/${m.audience_id}?tenant_id=${profile.tenant_id}`}>
                      {m.audience_id}
                    </a>
                  </td>
                  <td className="muted">{m.entered_at}</td>
                  <td className="muted">{m.exited_at ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Consent</h2>
        <p className="muted">
          Append-only — recording never overwrites history, current state is the latest
          non-expired event per purpose.
        </p>
        {!consent || consent.current.length === 0 ? (
          <p className="muted">No consent recorded.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Purpose</th>
                <th>Status</th>
                <th>Source</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {consent.current.map((c) => (
                <tr key={c.purpose}>
                  <td className="mono">{c.purpose}</td>
                  <td className={c.granted ? "" : "error"}>
                    {c.granted ? "granted" : "revoked"}
                  </td>
                  <td>{c.source}</td>
                  <td className="muted">{c.updated_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <form action={boundConsent}>
          <input type="hidden" name="tenant_id" value={profile.tenant_id} />
          <label>
            Purpose
            <input name="purpose" placeholder="email" required />
          </label>
          <label>
            <input type="checkbox" name="granted" defaultChecked /> Granted
          </label>
          <label>
            Source
            <input name="source" placeholder="preference-center" required />
          </label>
          <button type="submit">Record consent</button>
        </form>
      </section>

      <section>
        <h2>Journeys</h2>
        <p className="muted">
          Runs started automatically when this profile entered a trigger audience, or manually —
          see <a href="/journeys">the journey list</a> to create more.
        </p>
        {!journeyRuns || journeyRuns.length === 0 ? (
          <p className="muted">No journey runs for this profile.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Journey</th>
                <th>Status</th>
                <th>Current node</th>
                <th>Wakes at</th>
                <th>Started</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {journeyRuns.map((r) => (
                <tr key={r.id}>
                  <td className="mono">
                    <a href={`/journeys/${r.journey_id}?tenant_id=${profile.tenant_id}`}>
                      {r.journey_id}
                    </a>
                  </td>
                  <td>{r.status}</td>
                  <td className="mono">{r.current_node}</td>
                  <td className="muted">{r.wake_at ?? "—"}</td>
                  <td className="muted">{r.started_at}</td>
                  <td>
                    <a href={`/journeys/${r.journey_id}/runs/${r.id}`}>events</a>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>Event timeline</h2>
        {!events || events.length === 0 ? (
          <p className="muted">No events recorded.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Type</th>
                <th>Schema</th>
                <th>Identity claims</th>
                <th>Data</th>
                <th>Occurred</th>
              </tr>
            </thead>
            <tbody>
              {events.map((e) => (
                <tr key={e.id}>
                  <td>{e.event_type}</td>
                  <td className="mono">
                    {e.schema_name}@{e.schema_version}
                  </td>
                  <td className="mono">
                    {e.identity
                      .map((claim) => `${claim.namespace}:${claim.value}`)
                      .join(", ")}
                  </td>
                  <td className="mono">{JSON.stringify(e.data)}</td>
                  <td className="muted">{e.occurred_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
