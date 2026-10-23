package com.messagebirds.android

import java.io.File
import kotlin.math.min
import kotlin.math.pow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONArray
import org.json.JSONObject

private const val MAX_ATTEMPTS = 5

/**
 * JSON-file-backed queue, mirroring `sdk/web`'s `EventQueue`
 * (`sdk/web/src/queue.ts`) and `sdk/ios`'s `EventQueue`
 * (`sdk/ios/Sources/MessageBirdsAnalytics/EventQueue.swift`) exactly:
 * `POST /events` takes one envelope, not a batch, so `flush` sends
 * serially and stops at the first failure rather than reordering or
 * hammering a possibly-down endpoint. A `Mutex` serializes concurrent
 * `enqueue`/`flush` calls, the coroutine equivalent of Swift's `actor`.
 */
class EventQueue(
    private val file: File,
    private val send: suspend (EventEnvelope) -> Boolean,
) {
    private val mutex = Mutex()
    private var nextRetryAtMillis: Long = 0

    suspend fun enqueue(envelope: EventEnvelope) = mutex.withLock {
        val items = load().toMutableList()
        items.add(QueuedEvent(envelope, attempts = 0))
        persist(items)
    }

    /** @return true if the queue was fully drained. */
    suspend fun flush(onDropped: ((EventEnvelope) -> Unit)? = null): Boolean = mutex.withLock {
        if (System.currentTimeMillis() < nextRetryAtMillis) return@withLock false

        val items = load().toMutableList()
        var index = 0
        while (index < items.size) {
            val ok = send(items[index].envelope)
            if (ok) {
                index += 1
                continue
            }

            items[index] = items[index].copy(attempts = items[index].attempts + 1)
            if (items[index].attempts >= MAX_ATTEMPTS) {
                onDropped?.invoke(items[index].envelope)
                index += 1
                continue
            }

            nextRetryAtMillis = System.currentTimeMillis() + backoffMillis(items[index].attempts)
            persist(items.subList(index, items.size))
            return@withLock false
        }

        persist(emptyList())
        true
    }

    /** Test-only: bypasses the backoff delay so a retry test doesn't sleep for real. */
    internal suspend fun resetBackoffForTesting() = mutex.withLock {
        nextRetryAtMillis = 0
    }

    private fun load(): List<QueuedEvent> {
        if (!file.exists()) return emptyList()
        return try {
            val array = JSONArray(file.readText())
            (0 until array.length()).map { i ->
                val obj = array.getJSONObject(i)
                QueuedEvent(EventEnvelope.fromJson(obj.getJSONObject("envelope")), obj.getInt("attempts"))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private fun persist(items: List<QueuedEvent>) {
        val array = JSONArray()
        for (item in items) {
            array.put(
                JSONObject().apply {
                    put("envelope", item.envelope.toJson())
                    put("attempts", item.attempts)
                },
            )
        }
        file.writeText(array.toString())
    }

    private fun backoffMillis(attempts: Int): Long =
        min(30_000.0, 1_000.0 * 2.0.pow(attempts - 1)).toLong()
}

internal data class QueuedEvent(val envelope: EventEnvelope, val attempts: Int)
