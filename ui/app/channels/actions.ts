"use server";

import { redirect } from "next/navigation";
import { apiFetch, ApiError } from "@/lib/api";

export async function createChannel(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const kind = String(formData.get("kind") ?? "").trim();
  const name = String(formData.get("name") ?? "").trim();
  const url = String(formData.get("webhook_url") ?? "").trim();

  if (!tenantId || !kind || !name || !url) {
    redirect(
      `/channels?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "kind, name, and webhook URL are required",
      )}`,
    );
  }

  try {
    await apiFetch("/channels", {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, kind, name, config: { url } }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    redirect(`/channels?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  redirect(
    `/channels?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(`created channel "${name}"`)}`,
  );
}
