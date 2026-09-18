import Foundation

/// Mirrors `mb_core::IdentityRef` (see `sdk/js/src/types.ts`, `sdk/python/src/messagebirds/types.py`).
public struct IdentityRef: Codable, Equatable {
    public let namespace: String
    public let value: String
    public let primary: Bool?
    public let source: String
    public let confidence: Double?

    public init(namespace: String, value: String, primary: Bool? = nil, source: String, confidence: Double? = nil) {
        self.namespace = namespace
        self.value = value
        self.primary = primary
        self.source = source
        self.confidence = confidence
    }
}

/// Mirrors `mb_core::EventSource`.
public struct EventSource: Codable, Equatable {
    public let type: String
    public let name: String

    public init(type: String, name: String) {
        self.type = type
        self.name = name
    }
}

/// Mirrors `mb_core::SchemaRef`.
public struct SchemaRef: Codable, Equatable {
    public let name: String
    public let version: String

    public init(name: String, version: String) {
        self.name = name
        self.version = version
    }
}

/// A JSON value with no fixed shape — used for `data`/`context`, matching
/// `Record<string, unknown>` on the TS side and `dict[str, Any]` on the
/// Python side.
public enum JSONValue: Codable, Equatable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let v = try? container.decode(Bool.self) {
            self = .bool(v)
        } else if let v = try? container.decode(Double.self) {
            self = .number(v)
        } else if let v = try? container.decode(String.self) {
            self = .string(v)
        } else if let v = try? container.decode([String: JSONValue].self) {
            self = .object(v)
        } else if let v = try? container.decode([JSONValue].self) {
            self = .array(v)
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let v): try container.encode(v)
        case .number(let v): try container.encode(v)
        case .bool(let v): try container.encode(v)
        case .object(let v): try container.encode(v)
        case .array(let v): try container.encode(v)
        case .null: try container.encodeNil()
        }
    }
}

/// Mirrors `mb_core::EventEnvelope` — the JSON body `POST /events` expects.
public struct EventEnvelope: Codable, Equatable {
    public let event_id: String
    public let event_type: String
    public let timestamp: String
    public let source: EventSource
    public let identity: [IdentityRef]
    public let context: [String: JSONValue]?
    public let data: [String: JSONValue]?
    public let schema: SchemaRef
    public let tenant_id: String

    public init(
        event_id: String,
        event_type: String,
        timestamp: String,
        source: EventSource,
        identity: [IdentityRef],
        context: [String: JSONValue]? = nil,
        data: [String: JSONValue]? = nil,
        schema: SchemaRef,
        tenant_id: String
    ) {
        self.event_id = event_id
        self.event_type = event_type
        self.timestamp = timestamp
        self.source = source
        self.identity = identity
        self.context = context
        self.data = data
        self.schema = schema
        self.tenant_id = tenant_id
    }
}
