import type { EventDetail, MixinDef, ProfileView, SchemaDefinition } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";

/** `"core/person@1.0"` -> `{namespace: "core", name: "person", version: "1.0"}`. */
function parseMixinKey(key: string): { namespace: string; name: string; version: string } | null {
  const at = key.lastIndexOf("@");
  if (at === -1) return null;
  const version = key.slice(at + 1);
  const nsAndName = key.slice(0, at);
  const slash = nsAndName.indexOf("/");
  if (slash === -1) return null;
  return { namespace: nsAndName.slice(0, slash), name: nsAndName.slice(slash + 1), version };
}

/**
 * Traces a profile field back to where it came from: the winning
 * provenance record -> the source event -> its schema -> the mixin
 * definition it was composed into. Built entirely from data already
 * captured (Section 12's example chain, for the current winning value) —
 * not a ledger of every value ever observed, which is a different,
 * bigger feature.
 */
export default async function LineagePage({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ mixin?: string; field?: string }>;
}) {
  const { id } = await params;
  const { mixin, field } = await searchParams;

  if (!mixin || !field) {
    return (
      <main>
        <p className="error">Both `mixin` and `field` query parameters are required.</p>
      </main>
    );
  }

  const profile = await apiFetch<ProfileView>(`/profiles/${id}`);
  if (!profile) {
    return (
      <main>
        <p className="error">Profile {id} not found.</p>
      </main>
    );
  }

  const provenance = profile.provenance.find((p) => p.mixin_key === mixin && p.field_path === field);
  if (!provenance) {
    return (
      <main>
        <p className="error">
          No provenance recorded for <span className="mono">{mixin}.{field}</span> on this profile.
        </p>
      </main>
    );
  }

  const event = await apiFetch<EventDetail>(`/events/${provenance.event_id}`);
  const schema = event
    ? await apiFetch<SchemaDefinition>(`/schemas/${event.schema_name}/${event.schema_version}`)
    : null;

  const parsedMixin = parseMixinKey(mixin);
  const mixinDef = parsedMixin
    ? await apiFetch<MixinDef>(
        `/mixins/${parsedMixin.namespace}/${parsedMixin.name}/${parsedMixin.version}`,
      )
    : null;

  return (
    <main>
      <section>
        <h2>
          Lineage: <span className="mono">{mixin}.{field}</span>
        </h2>
        <p className="muted">
          <a href={`/profiles/${id}`}>back to profile</a>
        </p>

        <h3>1. Winning value</h3>
        <pre>{JSON.stringify(provenance, null, 2)}</pre>

        <h3>2. Source event</h3>
        {event ? (
          <pre>{JSON.stringify(event, null, 2)}</pre>
        ) : (
          <p className="muted">Event {provenance.event_id} is no longer available.</p>
        )}

        <h3>
          3. Schema {event && (
            <span className="mono">
              {event.schema_name}@{event.schema_version}
            </span>
          )}
        </h3>
        {schema ? (
          <pre>{JSON.stringify(schema.fields, null, 2)}</pre>
        ) : (
          <p className="muted">Schema not found.</p>
        )}

        <h3>
          4. Mixin definition <span className="mono">{mixin}</span>
        </h3>
        {mixinDef ? (
          <pre>{JSON.stringify(mixinDef.fields, null, 2)}</pre>
        ) : (
          <p className="muted">Mixin not found.</p>
        )}

        <h3>5. Merge policy applied</h3>
        <p className="mono">{provenance.applied_policy}</p>
      </section>
    </main>
  );
}
