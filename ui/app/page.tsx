import { lookupProfile } from "./actions";

export default async function HomePage({
  searchParams,
}: {
  searchParams: Promise<{ error?: string }>;
}) {
  const { error } = await searchParams;
  return (
    <main>
      <section>
        <h2>Find a profile</h2>
        <p className="muted">
          Look up a profile by an identity claim it&apos;s linked to — e.g. the{" "}
          <span className="mono">anonymous_id</span> a client generated for itself.
        </p>
        {error && <p className="error">{error}</p>}
        <form action={lookupProfile}>
          <label>
            Tenant ID
            <input name="tenant_id" required />
          </label>
          <label>
            Namespace
            <input name="namespace" defaultValue="anonymous_id" required />
          </label>
          <label>
            Value
            <input name="value" required />
          </label>
          <button type="submit">Look up</button>
        </form>
      </section>
    </main>
  );
}
