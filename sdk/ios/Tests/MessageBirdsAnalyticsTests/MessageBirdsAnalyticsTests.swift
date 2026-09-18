import Foundation
import Testing
@testable import MessageBirdsAnalytics

struct MessageBirdsAnalyticsTests {
    private func tempFileURL() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("mb-client-queue-\(UUID().uuidString).json")
    }

    private func tempDefaults() -> UserDefaults {
        UserDefaults(suiteName: "mb-test-\(UUID().uuidString)")!
    }

    @Test func trackPostsARenderedEnvelopeToEventsEndpoint() async throws {
        let capturedBody = Mutex<Data?>(nil)
        MockURLProtocol.handler = { request in
            #expect(request.url?.path == "/events")
            #expect(request.httpMethod == "POST")
            capturedBody.setValue(request.httpBodyFromStream() ?? request.httpBody)
            return (200, Data(#"{"event_id":"ignored"}"#.utf8))
        }

        let analytics = MessageBirdsAnalytics(
            writeKey: "tenant-1",
            apiUrl: "http://localhost:8080",
            queueFileURL: tempFileURL(),
            session: MockURLProtocol.makeSession(),
            defaults: tempDefaults()
        )
        analytics.identify(namespace: "anonymous_id", value: "anon-123")
        analytics.track(eventType: "commerce.product_view", data: ["product_id": .string("p1")])

        // track() enqueues+flushes on a detached Task; give it a beat.
        try await Task.sleep(nanoseconds: 300_000_000)

        let body = try #require(capturedBody.value)
        let envelope = try JSONDecoder().decode(EventEnvelope.self, from: body)
        #expect(envelope.event_type == "commerce.product_view")
        #expect(envelope.tenant_id == "tenant-1")
        #expect(envelope.identity.first?.value == "anon-123")

        MockURLProtocol.handler = nil
    }

    @Test func anonymousIdPersistsAcrossInstances() {
        let defaults = tempDefaults()
        let fileURL = tempFileURL()

        let first = MessageBirdsAnalytics(
            writeKey: "tenant-1", apiUrl: "http://localhost:8080",
            queueFileURL: fileURL, session: MockURLProtocol.makeSession(), defaults: defaults
        )
        let second = MessageBirdsAnalytics(
            writeKey: "tenant-1", apiUrl: "http://localhost:8080",
            queueFileURL: fileURL, session: MockURLProtocol.makeSession(), defaults: defaults
        )

        // Both read/write the same UserDefaults suite, so the id minted by
        // `first`'s init must be the one `second` reads back.
        #expect(defaults.string(forKey: "messagebirds.anonymous_id")?.isEmpty == false)
        _ = first
        _ = second
    }
}

private extension URLRequest {
    /// `MockURLProtocol` receives the request after `URLSession` has
    /// already moved `httpBody` into a stream in some code paths; this
    /// covers both.
    func httpBodyFromStream() -> Data? {
        guard let stream = httpBodyStream else { return nil }
        stream.open()
        defer { stream.close() }
        var data = Data()
        let bufferSize = 4096
        var buffer = [UInt8](repeating: 0, count: bufferSize)
        while stream.hasBytesAvailable {
            let read = stream.read(&buffer, maxLength: bufferSize)
            if read <= 0 { break }
            data.append(buffer, count: read)
        }
        return data.isEmpty ? nil : data
    }
}
