"use server";

import { redirect } from "next/navigation";
import { apiFetch, ApiError } from "@/lib/api";

export async function createTemplate(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const channelKind = String(formData.get("channel_kind") ?? "").trim();
  const name = String(formData.get("name") ?? "").trim();
  const subject = String(formData.get("subject") ?? "").trim();
  const body = String(formData.get("body") ?? "").trim();

  if (!tenantId || !channelKind || !name || !body) {
    redirect(
      `/templates?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "channel kind, name, and body are required",
      )}`,
    );
  }

  try {
    await apiFetch("/templates", {
      method: "POST",
      body: JSON.stringify({
        tenant_id: tenantId,
        channel_kind: channelKind,
        name,
        subject: subject === "" ? null : subject,
        body,
      }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    redirect(`/templates?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  redirect(
    `/templates?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(
      `created template "${name}" v1 (or next version if the name already existed)`,
    )}`,
  );
}
