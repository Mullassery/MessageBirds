"use server";

import { redirect } from "next/navigation";
import type { ProfileView } from "@messagebirds/sdk";
import { apiFetch, ApiError } from "@/lib/api";

export async function lookupProfile(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const namespace = String(formData.get("namespace") ?? "").trim();
  const value = String(formData.get("value") ?? "").trim();

  if (!tenantId || !namespace || !value) {
    redirect(`/?error=${encodeURIComponent("all three fields are required")}`);
  }

  const params = new URLSearchParams({ tenant_id: tenantId, namespace, value });
  let profile: ProfileView | null;
  try {
    profile = await apiFetch<ProfileView>(`/profiles/by-identity?${params}`);
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "lookup failed";
    redirect(`/?error=${encodeURIComponent(message)}`);
  }

  if (!profile) {
    redirect(`/?error=${encodeURIComponent("no profile is linked to that identity claim")}`);
  }

  redirect(`/profiles/${profile.id}`);
}
