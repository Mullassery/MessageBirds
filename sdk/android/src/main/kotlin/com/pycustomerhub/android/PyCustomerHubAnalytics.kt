package com.messagebirds.android

import android.content.Context
import android.content.SharedPreferences
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.json.JSONObject

private const val PREFS_NAME = "messagebirds"
private const val ANONYMOUS_ID_KEY = "anonymous_id"

/**
 * Real event collector for Android, mirroring `sdk/web`'s
 * `MessageBirdsWebAnalytics` and `sdk/ios`'s `MessageBirdsAnalytics`:
 * queue-and-retry over `POST /events` (no batch endpoint exists to send
 * a batch to), one real HTTP call per event via `HttpURLConnection` (no
 * OkHttp dependency — keeps this module dependency-free beyond Kotlin
 * stdlib + coroutines).
 *
 * NOTE ON VERIFICATION: this file is written against real, standard
 * Android/Kotlin APIs, but this environment has no `gradle`/`kotlinc`/
 * Android SDK, so — unlike `sdk/web` and `sdk/ios` — it has **not** been
 * compiled or unit-tested here. See `docs/ARCHITECTURE.md`'s Phase 6
 * section. Build with `./gradlew build` (wrapper not committed — see
 * `sdk/android/README.md`) in an environment with the Android SDK before
 * relying on it.
 */
class MessageBirdsAnalytics(
    context: Context,
    private val tenantId: String,
    private val apiUrl: String,
    private val scope: CoroutineScope = CoroutineScope(Dispatchers.IO),
    queueFile: File? = null,
    private val prefs: SharedPreferences = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE),
) {
    private val queue = EventQueue(
        queueFile ?: File(context.filesDir, "messagebirds-queue.json"),
        ::sendOnce,
    )
    private val anonymousId: String = loadOrCreateAnonymousId()
    private var identity: List<IdentityRef> = emptyList()

    fun identify(namespace: String, value: String) {
        identity = listOf(IdentityRef(namespace, value, primary = true, source = "android-sdk"))
    }

    fun track(
        eventType: String,
        data: JSONObject? = null,
        schema: SchemaRef? = null,
        context: JSONObject? = null,
    ) {
        val eventIdentity = identity.ifEmpty {
            listOf(IdentityRef("anonymous_id", anonymousId, primary = true, source = "android-sdk"))
        }

        val envelope = EventEnvelope(
            eventId = UUID.randomUUID().toString(),
            eventType = eventType,
            timestamp = java.time.Instant.now().toString(),
            source = EventSource(type = "android", name = "android-sdk"),
            identity = eventIdentity,
            context = context,
            data = data,
            schema = schema ?: SchemaRef(name = eventType, version = "1.0"),
            tenantId = tenantId,
        )

        scope.launch {
            queue.enqueue(envelope)
            flush()
        }
    }

    suspend fun flush(): Boolean = queue.flush { dropped ->
        android.util.Log.w("messagebirds", "dropped event ${dropped.eventId} after repeated failures")
    }

    private fun sendOnce(envelope: EventEnvelope): Boolean {
        return try {
            val url = URL("${apiUrl.trimEnd('/')}/events")
            val connection = url.openConnection() as HttpURLConnection
            connection.requestMethod = "POST"
            connection.setRequestProperty("content-type", "application/json")
            connection.doOutput = true
            connection.outputStream.use { it.write(envelope.toJson().toString().toByteArray()) }
            val code = connection.responseCode
            connection.disconnect()
            code in 200..299
        } catch (e: Exception) {
            false
        }
    }

    private fun loadOrCreateAnonymousId(): String {
        prefs.getString(ANONYMOUS_ID_KEY, null)?.let { return it }
        val id = "anon-${UUID.randomUUID()}"
        prefs.edit().putString(ANONYMOUS_ID_KEY, id).apply()
        return id
    }
}
