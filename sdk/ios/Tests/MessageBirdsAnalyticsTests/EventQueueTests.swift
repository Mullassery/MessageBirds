import Testing
@testable import MessageBirdsAnalytics

struct EventQueueTests {
    private func tempFileURL() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("mb-queue-\(UUID().uuidString).json")
    }

    private func envelope(_ id: String) -> EventEnvelope {
        EventEnvelope(
            event_id: id,
            event_type: "test.event",
            timestamp: ISO8601DateFormatter().string(from: Date()),
            source: EventSource(type: "ios", name: "test"),
            identity: [],
            schema: SchemaRef(name: "test.event", version: "1.0"),
            tenant_id: "tenant-1"
        )
    }

    @Test func sendsQueuedEventAndClearsOnSuccess() async {
        let queue = EventQueue(fileURL: tempFileURL()) { _ in true }
        await queue.enqueue(envelope("a"))
        let ok = await queue.flush()
        #expect(ok)
    }

    @Test func sendsEventsInOrder() async {
        let sentIds = Mutex<[String]>([])
        let queue = EventQueue(fileURL: tempFileURL()) { e in
            sentIds.append(e.event_id)
            return true
        }
        await queue.enqueue(envelope("a"))
        await queue.enqueue(envelope("b"))
        await queue.enqueue(envelope("c"))
        _ = await queue.flush()

        #expect(sentIds.value == ["a", "b", "c"])
    }

    @Test func stopsAtFirstFailureAndKeepsLaterEventsQueued() async {
        let callCount = Mutex<Int>(0)
        let queue = EventQueue(fileURL: tempFileURL()) { _ in
            let n = callCount.increment()
            return n == 1 // "a" succeeds, "b" fails, "c" never attempted
        }
        await queue.enqueue(envelope("a"))
        await queue.enqueue(envelope("b"))
        await queue.enqueue(envelope("c"))
        let ok = await queue.flush()

        #expect(!ok)
        #expect(callCount.value == 2)
    }

    @Test func dropsEventAfterMaxAttemptsAndReportsIt() async {
        let queue = EventQueue(fileURL: tempFileURL()) { _ in false }
        await queue.enqueue(envelope("a"))

        let dropped = Mutex<[EventEnvelope]>([])
        for _ in 0..<5 {
            _ = await queue.flush(onDropped: { dropped.append($0) })
            await queue.resetBackoffForTesting()
        }

        #expect(dropped.value.map(\.event_id) == ["a"])
    }
}

/// Minimal thread-safe box — the closures above run off the actor, so
/// plain captured `var`s would be a data race; `NSLock`-backed rather than
/// pulling in a dependency for one test helper.
final class Mutex<T>: @unchecked Sendable {
    private var _value: T
    private let lock = NSLock()

    init(_ value: T) { self._value = value }

    var value: T {
        lock.lock(); defer { lock.unlock() }
        return _value
    }

    func setValue(_ newValue: T) {
        lock.lock(); defer { lock.unlock() }
        _value = newValue
    }

    func append<Element>(_ element: Element) where T == [Element] {
        lock.lock(); defer { lock.unlock() }
        _value.append(element)
    }

    @discardableResult
    func increment() -> Int where T == Int {
        lock.lock(); defer { lock.unlock() }
        _value += 1
        return _value
    }
}
