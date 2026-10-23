import Foundation

struct QueuedEvent: Codable {
    var envelope: EventEnvelope
    var attempts: Int
}

/// JSON-file-backed queue, mirroring `sdk/web`'s `EventQueue`
/// (`sdk/web/src/queue.ts`) design exactly: `POST /events` takes one
/// envelope, not a batch, so `flush` sends serially and stops at the
/// first failure rather than reordering or hammering a possibly-down
/// endpoint. An `actor` (not a class + locks) since queue mutation must be
/// serialized across concurrent `track`/`flush` calls.
actor EventQueue {
    private let fileURL: URL
    private let send: (EventEnvelope) async -> Bool
    private var nextRetryAt: Date = .distantPast
    private static let maxAttempts = 5

    init(fileURL: URL, send: @escaping (EventEnvelope) async -> Bool) {
        self.fileURL = fileURL
        self.send = send
    }

    func enqueue(_ envelope: EventEnvelope) {
        var items = load()
        items.append(QueuedEvent(envelope: envelope, attempts: 0))
        persist(items)
    }

    @discardableResult
    func flush(onDropped: ((EventEnvelope) -> Void)? = nil) async -> Bool {
        if Date() < nextRetryAt { return false }

        var items = load()
        var index = 0
        while index < items.count {
            let ok = await send(items[index].envelope)
            if ok {
                index += 1
                continue
            }

            items[index].attempts += 1
            if items[index].attempts >= Self.maxAttempts {
                onDropped?(items[index].envelope)
                index += 1
                continue
            }

            nextRetryAt = Date().addingTimeInterval(Self.backoffSeconds(attempts: items[index].attempts))
            persist(Array(items[index...]))
            return false
        }

        persist([])
        return true
    }

    private func load() -> [QueuedEvent] {
        guard let data = try? Data(contentsOf: fileURL) else { return [] }
        return (try? JSONDecoder().decode([QueuedEvent].self, from: data)) ?? []
    }

    private func persist(_ items: [QueuedEvent]) {
        guard let data = try? JSONEncoder().encode(items) else { return }
        try? data.write(to: fileURL, options: .atomic)
    }

    private static func backoffSeconds(attempts: Int) -> TimeInterval {
        min(30, pow(2, Double(attempts - 1)))
    }

    /// Test-only: bypasses the backoff delay so a retry test doesn't need
    /// to sleep for real. Not part of the SDK's real usage surface.
    func resetBackoffForTesting() {
        nextRetryAt = .distantPast
    }
}
