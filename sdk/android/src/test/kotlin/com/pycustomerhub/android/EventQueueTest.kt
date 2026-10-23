package com.messagebirds.android

import java.io.File
import java.util.UUID
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Mirrors `sdk/web/tests/queue.test.ts` and
 * `sdk/ios/Tests/.../EventQueueTests.swift` — same scenarios, same
 * queue/retry/backoff contract. NOT run in this session (no
 * `gradle`/Android SDK here); written for CI / a machine with the
 * Android SDK installed. See the note on `MessageBirdsAnalytics`.
 */
class EventQueueTest {
    private fun tempFile(): File = File.createTempFile("mb-queue-${UUID.randomUUID()}", ".json")

    private fun envelope(id: String): EventEnvelope = EventEnvelope(
        eventId = id,
        eventType = "test.event",
        timestamp = java.time.Instant.now().toString(),
        source = EventSource("android", "test"),
        identity = emptyList(),
        schema = SchemaRef("test.event", "1.0"),
        tenantId = "tenant-1",
    )

    @Test
    fun sendsQueuedEventAndClearsOnSuccess() = runTest {
        val queue = EventQueue(tempFile()) { true }
        queue.enqueue(envelope("a"))
        assertTrue(queue.flush())
    }

    @Test
    fun sendsEventsInOrder() = runTest {
        val sentIds = mutableListOf<String>()
        val queue = EventQueue(tempFile()) { e -> sentIds.add(e.eventId); true }
        queue.enqueue(envelope("a"))
        queue.enqueue(envelope("b"))
        queue.enqueue(envelope("c"))
        queue.flush()

        assertEquals(listOf("a", "b", "c"), sentIds)
    }

    @Test
    fun stopsAtFirstFailureAndKeepsLaterEventsQueued() = runTest {
        var callCount = 0
        val queue = EventQueue(tempFile()) { callCount += 1; callCount == 1 }
        queue.enqueue(envelope("a"))
        queue.enqueue(envelope("b"))
        queue.enqueue(envelope("c"))
        val drained = queue.flush()

        assertFalse(drained)
        assertEquals(2, callCount)
    }

    @Test
    fun dropsEventAfterMaxAttemptsAndReportsIt() = runTest {
        val queue = EventQueue(tempFile()) { false }
        queue.enqueue(envelope("a"))

        val dropped = mutableListOf<EventEnvelope>()
        repeat(5) {
            queue.flush { dropped.add(it) }
            queue.resetBackoffForTesting()
        }

        assertEquals(listOf("a"), dropped.map { it.eventId })
    }
}
