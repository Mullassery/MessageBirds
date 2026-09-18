import Foundation
#if canImport(UIKit)
import UIKit
#endif

public final class MessageBirdsAnalytics {
    private let tenantId: String
    private let apiUrl: URL
    private let session: URLSession
    private let queue: EventQueue
    private let anonymousId: String
    private var identity: [IdentityRef] = []
    private let defaults: UserDefaults

    private static let anonymousIdKey = "messagebirds.anonymous_id"

    /// - Parameters:
    ///   - writeKey: The tenant to send events under. No real per-tenant
    ///     write-key issuance exists yet (see docs/ARCHITECTURE.md) — this
    ///     is the tenant_id itself until that's built.
    ///   - apiUrl: Base URL of `mb-api` (or the edge ingestion gateway).
    ///   - queueFileURL: Where the offline queue persists between
    ///     launches. Defaults to a file in the app's Documents directory.
    ///   - session: Injectable for tests (see `MessageBirdsAnalyticsTests`).
    ///   - defaults: Injectable for tests, so the anonymous id doesn't leak
    ///     across test runs via the real `UserDefaults.standard`.
    public init(
        writeKey: String,
        apiUrl: String,
        queueFileURL: URL? = nil,
        session: URLSession = .shared,
        defaults: UserDefaults = .standard
    ) {
        self.tenantId = writeKey
        self.apiUrl = URL(string: apiUrl)!
        self.session = session
        self.defaults = defaults

        let fileURL = queueFileURL ?? Self.defaultQueueFileURL()
        self.queue = EventQueue(fileURL: fileURL) { [session] envelope in
            await Self.sendOnce(envelope, apiUrl: URL(string: apiUrl)!, session: session)
        }

        if let existing = defaults.string(forKey: Self.anonymousIdKey) {
            self.anonymousId = existing
        } else {
            let id = "anon-\(UUID().uuidString)"
            defaults.set(id, forKey: Self.anonymousIdKey)
            self.anonymousId = id
        }

        #if canImport(UIKit)
        NotificationCenter.default.addObserver(
            forName: UIApplication.didEnterBackgroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { await self?.flush() }
        }
        #endif
    }

    public func identify(namespace: String, value: String) {
        identity = [IdentityRef(namespace: namespace, value: value, primary: true, source: "ios-sdk")]
    }

    public func track(
        eventType: String,
        data: [String: JSONValue] = [:],
        schema: SchemaRef? = nil,
        context: [String: JSONValue]? = nil
    ) {
        let eventIdentity = identity.isEmpty
            ? [IdentityRef(namespace: "anonymous_id", value: anonymousId, primary: true, source: "ios-sdk")]
            : identity

        let envelope = EventEnvelope(
            event_id: UUID().uuidString,
            event_type: eventType,
            timestamp: ISO8601DateFormatter().string(from: Date()),
            source: EventSource(type: "ios", name: "ios-sdk"),
            identity: eventIdentity,
            context: context,
            data: data,
            schema: schema ?? SchemaRef(name: eventType, version: "1.0"),
            tenant_id: tenantId
        )

        Task {
            await queue.enqueue(envelope)
            await flush()
        }
    }

    @discardableResult
    public func flush() async -> Bool {
        await queue.flush { envelope in
            print("[messagebirds] dropped event \(envelope.event_id) after repeated failures")
        }
    }

    private static func sendOnce(_ envelope: EventEnvelope, apiUrl: URL, session: URLSession) async -> Bool {
        var request = URLRequest(url: apiUrl.appendingPathComponent("events"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "content-type")
        guard let body = try? JSONEncoder().encode(envelope) else { return false }
        request.httpBody = body

        do {
            let (_, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse else { return false }
            return (200..<300).contains(http.statusCode)
        } catch {
            return false
        }
    }

    private static func defaultQueueFileURL() -> URL {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        return dir.appendingPathComponent("messagebirds-queue.json")
    }
}
