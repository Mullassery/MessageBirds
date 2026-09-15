import type { AudienceDefinition, Destination } from "@messagebirds/sdk";
import { apiFetch } from "@/lib/api";
import { activateAudience, createAudience, createDestination } from "./actions";

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

export default async function AudiencesPage({
  searchParams,
}: {
  searchParams: Promise<{ tenant_id?: string; error?: string; note?: string }>;
}) {
  const { tenant_id: tenantId, error, note } = await searchParams;

  const [audiences, destinations] = tenantId
    ? await Promise.all([
        apiFetch<AudienceDefinition[]>(`/audiences?tenant_id=${encodeURIComponent(tenantId)}`),
        apiFetch<Destination[]>(`/destinations?tenant_id=${encodeURIComponent(tenantId)}`),
      ])
    : [null, null];

  return (
    <main>
      <section>
        <h2>Audiences</h2>
        <p className="muted">
          Rule-based segments, evaluated in real time as profiles update. Membership never
          activates anywhere automatically — see each audience&apos;s activate action below.
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
            <h2>Create an audience</h2>
            <p className="muted">
              One attribute condition and/or one event condition, combined with AND — not a full
              condition-tree editor. Build richer trees via <span className="mono">POST /audiences</span>{" "}
              directly.
            </p>
            <form action={createAudience}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Name
                <input name="name" required />
              </label>

              <fieldset>
                <legend>
                  <label>
                    <input type="checkbox" name="use_attribute" /> Attribute condition
                  </label>
                </legend>
                <label>
                  Mixin
                  <input name="attribute_mixin" placeholder="core/person@1.0" />
                </label>
                <label>
                  Field
                  <input name="attribute_field" placeholder="first_name" />
                </label>
                <label>
                  Operator
                  <select name="attribute_op" defaultValue="exists">
                    <option value="exists">exists</option>
                    <option value="not_exists">not_exists</option>
                    <option value="equals">equals</option>
                    <option value="not_equals">not_equals</option>
                    <option value="greater_than">greater_than</option>
                    <option value="less_than">less_than</option>
                    <option value="contains">contains</option>
                  </select>
                </label>
                <label>
                  Value (blank for exists/not_exists)
                  <input name="attribute_value" />
                </label>
              </fieldset>

              <fieldset>
                <legend>
                  <label>
                    <input type="checkbox" name="use_event" /> Event condition
                  </label>
                </legend>
                <label>
                  Event type
                  <input name="event_type" placeholder="commerce.product_view" />
                </label>
                <label>
                  Within days
                  <input name="within_days" type="number" defaultValue={7} />
                </label>
                <label>
                  Minimum count
                  <input name="min_count" type="number" defaultValue={1} />
                </label>
              </fieldset>

              <button type="submit">Create</button>
            </form>
          </section>

          <section>
            <h2>Destinations</h2>
            <p className="muted">Webhook is the only connector implemented so far.</p>
            {destinations && destinations.length > 0 && (
              <ul>
                {destinations.map((d) => (
                  <li key={d.id}>
                    {d.name} — <span className="mono">{String(d.config.url ?? "")}</span>{" "}
                    <span className="muted">[{d.supported_actions.join(", ") || "no actions declared"}]</span>
                  </li>
                ))}
              </ul>
            )}
            <form action={createDestination}>
              <input type="hidden" name="tenant_id" value={tenantId} />
              <label>
                Name
                <input name="destination_name" required />
              </label>
              <label>
                Webhook URL
                <input name="webhook_url" type="url" required />
              </label>
              <fieldset>
                <legend>Supported marketing actions</legend>
                {MARKETING_ACTIONS.map((a) => (
                  <label key={a} style={{ flexDirection: "row", gap: "0.4rem" }}>
                    <input type="checkbox" name="supported_actions" value={a} />
                    {a}
                  </label>
                ))}
              </fieldset>
              <button type="submit">Add destination</button>
            </form>
          </section>

          <section>
            <h2>Existing audiences</h2>
            {!audiences || audiences.length === 0 ? (
              <p className="muted">No audiences yet for this tenant.</p>
            ) : (
              audiences.map((a) => (
                <div key={a.id} style={{ marginBlock: "1rem" }}>
                  <h3>
                    {a.name} <span className="muted">v{a.version}</span>
                  </h3>
                  <p className="muted">
                    {a.status} · <a href={`/audiences/${a.id}?tenant_id=${tenantId}`}>view members</a>
                  </p>
                  <pre>{JSON.stringify(a.conditions, null, 2)}</pre>

                  {destinations && destinations.length > 0 && (
                    <form action={activateAudience}>
                      <input type="hidden" name="tenant_id" value={tenantId} />
                      <input type="hidden" name="audience_id" value={a.id} />
                      <label>
                        Destination
                        <select name="destination_id" required>
                          {destinations.map((d) => (
                            <option key={d.id} value={d.id}>
                              {d.name} ({d.kind})
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        Marketing action
                        <select name="action" required>
                          {MARKETING_ACTIONS.map((act) => (
                            <option key={act} value={act}>
                              {act}
                            </option>
                          ))}
                        </select>
                      </label>
                      <button type="submit">Activate</button>
                    </form>
                  )}
                </div>
              ))
            )}
          </section>
        </>
      )}
    </main>
  );
}
