"use server";

import { redirect } from "next/navigation";
import type { ActivationSummary, AttributeOp, Condition } from "@messagebirds/sdk";
import { apiFetch, ApiError } from "@/lib/api";

function parseAttributeValue(raw: FormDataEntryValue | null): unknown {
  const s = String(raw ?? "").trim();
  if (s === "") return undefined;
  if (s === "true") return true;
  if (s === "false") return false;
  if (s !== "" && !Number.isNaN(Number(s))) return Number(s);
  return s;
}

export async function createAudience(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const name = String(formData.get("name") ?? "").trim();

  const conditions: Condition[] = [];

  if (formData.get("use_attribute") === "on") {
    conditions.push({
      attribute: {
        mixin: String(formData.get("attribute_mixin") ?? "").trim(),
        field: String(formData.get("attribute_field") ?? "").trim(),
        op: String(formData.get("attribute_op") ?? "exists") as AttributeOp,
        value: parseAttributeValue(formData.get("attribute_value")),
      },
    });
  }

  if (formData.get("use_event") === "on") {
    conditions.push({
      event: {
        event_type: String(formData.get("event_type") ?? "").trim(),
        within_days: Number(formData.get("within_days") ?? 7),
        min_count: Number(formData.get("min_count") ?? 1),
      },
    });
  }

  if (!tenantId || !name || conditions.length === 0) {
    redirect(
      `/audiences?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "tenant, name, and at least one condition are required",
      )}`,
    );
  }

  const condition: Condition = conditions.length === 1 ? conditions[0] : { and: conditions };

  try {
    await apiFetch("/audiences", {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, name, conditions: condition }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    redirect(`/audiences?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  redirect(
    `/audiences?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(`created audience "${name}"`)}`,
  );
}

export async function createDestination(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const name = String(formData.get("destination_name") ?? "").trim();
  const url = String(formData.get("webhook_url") ?? "").trim();
  const supportedActions = formData.getAll("supported_actions").map(String);

  if (!tenantId || !name || !url) {
    redirect(
      `/audiences?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "destination name and webhook URL are required",
      )}`,
    );
  }

  try {
    await apiFetch("/destinations", {
      method: "POST",
      body: JSON.stringify({
        tenant_id: tenantId,
        kind: "webhook",
        name,
        config: { url },
        supported_actions: supportedActions,
      }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    redirect(`/audiences?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  redirect(
    `/audiences?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(
      `created destination "${name}"`,
    )}`,
  );
}

export async function activateAudience(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const audienceId = String(formData.get("audience_id") ?? "").trim();
  const destinationId = String(formData.get("destination_id") ?? "").trim();
  const action = String(formData.get("action") ?? "").trim();

  let summary: ActivationSummary | null;
  try {
    summary = await apiFetch<ActivationSummary>(`/audiences/${audienceId}/activate`, {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, destination_id: destinationId, action }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "activation failed";
    redirect(`/audiences?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);
  }

  const blockedCount = summary?.blocked.length ?? 0;
  redirect(
    `/audiences?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(
      `activation sent=${summary?.sent ?? 0} failed=${summary?.failed ?? 0} blocked=${blockedCount}` +
        (blockedCount > 0
          ? ` — first reason: ${JSON.stringify(summary?.blocked[0]?.reasons[0])}`
          : ""),
    )}`,
  );
}
