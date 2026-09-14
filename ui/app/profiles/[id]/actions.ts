"use server";

import { redirect } from "next/navigation";
import { apiFetch, ApiError } from "@/lib/api";

export async function mergeProfile(profileId: string, formData: FormData) {
  const fromProfileId = String(formData.get("from_profile_id") ?? "").trim();

  try {
    await apiFetch(`/profiles/${profileId}/merge`, {
      method: "POST",
      body: JSON.stringify({ from_profile_id: fromProfileId }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "merge failed";
    redirect(`/profiles/${profileId}?error=${encodeURIComponent(message)}`);
  }

  redirect(`/profiles/${profileId}?note=${encodeURIComponent(`merged ${fromProfileId} into this profile`)}`);
}

export async function splitIdentity(profileId: string, formData: FormData) {
  const namespace = String(formData.get("namespace") ?? "").trim();
  const value = String(formData.get("value") ?? "").trim();

  let result: { new_profile_id: string; note: string } | null;
  try {
    result = await apiFetch<{ new_profile_id: string; note: string }>(
      `/profiles/${profileId}/split`,
      {
        method: "POST",
        body: JSON.stringify({ namespace, value }),
      },
    );
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "split failed";
    redirect(`/profiles/${profileId}?error=${encodeURIComponent(message)}`);
  }

  const note = result
    ? `split ${namespace} off into new profile ${result.new_profile_id} — ${result.note}`
    : "split completed";
  redirect(`/profiles/${profileId}?note=${encodeURIComponent(note)}`);
}
