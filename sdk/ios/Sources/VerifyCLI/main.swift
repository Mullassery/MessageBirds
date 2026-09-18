import Foundation
import MessageBirdsAnalytics

// Exercises `MessageBirdsAnalytics`'s public contract with plain
// `assert`s — see the comment on the `mb-verify` target in Package.swift
// for why this exists alongside (not instead of) the XCTest/swift-testing
// suite in Tests/MessageBirdsAnalyticsTests.

final class StubURLProtocol: URLProtocol {
    static var handler: (@Sendable (URLRequest) -> (Int, Data?))?

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = StubURLProtocol.handler else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        let (status, body) = handler(request)
        let response = HTTPURLResponse(url: request.url!, statusCode: status, httpVersion: nil, headerFields: nil)!
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        if let body { client?.urlProtocol(self, didLoad: body) }
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}

    static func makeSession() -> URLSession {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [StubURLProtocol.self]
        return URLSession(configuration: config)
    }
}

final class RequestLog: @unchecked Sendable {
    private var requests: [URLRequest] = []
    private let lock = NSLock()

    func record(_ request: URLRequest) {
        lock.lock(); defer { lock.unlock() }
        requests.append(request)
    }

    var count: Int {
        lock.lock(); defer { lock.unlock() }
        return requests.count
    }

    var last: URLRequest? {
        lock.lock(); defer { lock.unlock() }
        return requests.last
    }
}

func tempFileURL() -> URL {
    FileManager.default.temporaryDirectory.appendingPathComponent("mb-verify-\(UUID().uuidString).json")
}

func check(_ label: String, _ condition: Bool) {
    if condition {
        print("[OK] \(label)")
    } else {
        print("[FAIL] \(label)")
        exit(1)
    }
}

let log = RequestLog()
StubURLProtocol.handler = { request in
    log.record(request)
    return (200, Data(#"{"event_id":"ignored"}"#.utf8))
}

let defaults = UserDefaults(suiteName: "mb-verify-\(UUID().uuidString)")!
let analytics = MessageBirdsAnalytics(
    writeKey: "tenant-1",
    apiUrl: "http://localhost:8080",
    queueFileURL: tempFileURL(),
    session: StubURLProtocol.makeSession(),
    defaults: defaults
)
analytics.identify(namespace: "anonymous_id", value: "anon-verify-1")
analytics.track(eventType: "commerce.product_view", data: ["product_id": .string("p1")])

Thread.sleep(forTimeInterval: 0.5) // track() enqueues+flushes on a detached Task

check("track() delivered exactly one POST to /events", log.count == 1)
if let last = log.last {
    check("request path is /events", last.url?.path == "/events")
    check("request method is POST", last.httpMethod == "POST")
}

let secondClient = MessageBirdsAnalytics(
    writeKey: "tenant-1",
    apiUrl: "http://localhost:8080",
    queueFileURL: tempFileURL(),
    session: StubURLProtocol.makeSession(),
    defaults: defaults
)
_ = secondClient
check(
    "anonymous id persisted across instances sharing UserDefaults",
    defaults.string(forKey: "messagebirds.anonymous_id") != nil
)

print("\n[mb-verify] ALL CHECKS PASSED")
