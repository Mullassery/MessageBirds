import type { EventEnvelope, ProfileView } from "./types.js";

export interface MessageBirdsClientOptions {
  baseUrl: string;
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export class MessageBirdsClient {
  private readonly baseUrl: string;

  constructor(options: MessageBirdsClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/$/, "");
  }

  async sendEvent(envelope: EventEnvelope): Promise<{ event_id: string }> {
    const res = await fetch(`${this.baseUrl}/events`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(envelope),
    });
    if (!res.ok) {
      throw new ApiError(res.status, `POST /events failed: ${await res.text()}`);
    }
    return (await res.json()) as { event_id: string };
  }

  async getProfile(profileId: string): Promise<ProfileView | null> {
    const res = await fetch(`${this.baseUrl}/profiles/${profileId}`);
    if (res.status === 404) return null;
    if (!res.ok) {
      throw new ApiError(res.status, `GET /profiles/${profileId} failed: ${await res.text()}`);
    }
    return (await res.json()) as ProfileView;
  }

  async findProfileByIdentity(
    tenantId: string,
    namespace: string,
    value: string,
  ): Promise<ProfileView | null> {
    const params = new URLSearchParams({ tenant_id: tenantId, namespace, value });
    const res = await fetch(`${this.baseUrl}/profiles/by-identity?${params}`);
    if (res.status === 404) return null;
    if (!res.ok) {
      throw new ApiError(res.status, `GET /profiles/by-identity failed: ${await res.text()}`);
    }
    return (await res.json()) as ProfileView;
  }
}

export type {
  ActivationSummary,
  AttributeOp,
  AudienceDefinition,
  AudienceStatus,
  BlockedProfile,
  Channel,
  Condition,
  ConsentEvent,
  ConsentState,
  ConsentView,
  ContactPolicy,
  DeadLetterEventView,
  Destination,
  Effect,
  EventDetail,
  EventEnvelope,
  EventSource,
  EventSummary,
  FieldDef,
  FieldProvenance,
  FieldType,
  IdentityAuditEntry,
  IdentityAuditKind,
  IdentityNode,
  IdentityRef,
  IdentityView,
  JourneyDefinition,
  JourneyRun,
  JourneyRunEvent,
  JourneyStatus,
  Membership,
  MembershipKind,
  MergeSuggestion,
  MessageTemplate,
  MixinDef,
  MixinStatus,
  Node,
  NodeKind,
  Policy,
  PolicySimulateResponse,
  ProfileView,
  Reason,
  RenderedTemplate,
  RunEventKind,
  RunStatus,
  SchemaDefinition,
  SchemaRef,
  SchemaStatus,
  SplitBranch,
  Trigger,
} from "./types.js";
