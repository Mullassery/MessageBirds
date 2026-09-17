"use server";

import { redirect } from "next/navigation";
import type { Node, Trigger } from "@messagebirds/sdk";
import { apiFetch, ApiError } from "@/lib/api";

export async function createJourney(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const name = String(formData.get("name") ?? "").trim();
  const entryNode = String(formData.get("entry_node") ?? "").trim();
  const triggerRaw = String(formData.get("trigger") ?? "").trim();
  const nodesRaw = String(formData.get("nodes") ?? "").trim();

  const fail = (message: string) =>
    redirect(`/journeys?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`);

  if (!tenantId || !name || !entryNode) {
    fail("tenant, name, and entry_node are required");
  }

  let trigger: Trigger;
  let nodes: Node[];
  try {
    trigger = JSON.parse(triggerRaw) as Trigger;
  } catch {
    fail("trigger is not valid JSON");
    return;
  }
  try {
    nodes = JSON.parse(nodesRaw) as Node[];
  } catch {
    fail("nodes is not valid JSON");
    return;
  }

  try {
    await apiFetch("/journeys", {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, name, trigger, nodes, entry_node: entryNode }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "create failed";
    fail(message);
  }

  redirect(
    `/journeys?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(`created journey "${name}"`)}`,
  );
}

export async function startJourney(formData: FormData) {
  const tenantId = String(formData.get("tenant_id") ?? "").trim();
  const journeyId = String(formData.get("journey_id") ?? "").trim();
  const profileId = String(formData.get("profile_id") ?? "").trim();

  if (!profileId) {
    redirect(
      `/journeys/${journeyId}?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(
        "profile_id is required",
      )}`,
    );
  }

  try {
    await apiFetch(`/journeys/${journeyId}/start`, {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId }),
    });
  } catch (err) {
    const message = err instanceof ApiError ? err.message : "start failed";
    redirect(
      `/journeys/${journeyId}?tenant_id=${encodeURIComponent(tenantId)}&error=${encodeURIComponent(message)}`,
    );
  }

  redirect(
    `/journeys/${journeyId}?tenant_id=${encodeURIComponent(tenantId)}&note=${encodeURIComponent(
      `enrolled profile ${profileId}`,
    )}`,
  );
}
