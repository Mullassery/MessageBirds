"use server";

import { redirect } from "next/navigation";
import type { Effect } from "@messagebirds/sdk";
import { apiFetch, ApiError } from "@/lib/api";

export async function createPolicy(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const label = String(formData.get("label") ?? "").trim();
  const action = String(formData.get("action") ?? "").trim();
  const effect = String(formData.get("effect") ?? "deny") as Effect;
  const priority = Number(formData.get("priority") ?? 0);

  if (!tenantId || !label || !action) {
    redirect(
      `/governance?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "tenant, label, and action are required",
      )}`,
    );
  }

  try {
    await apiFetch("/policies", {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, label, action, effect, priority }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    redirect(`/governance?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  redirect(
    `/governance?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(
      `created policy: ${effect} ${label} for ${action}`,
    )}`,
  );
}
