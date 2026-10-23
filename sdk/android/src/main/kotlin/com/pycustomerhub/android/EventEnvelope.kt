package com.messagebirds.android

import org.json.JSONArray
import org.json.JSONObject

/** Mirrors `mb_core::IdentityRef` (see `sdk/js/src/types.ts`, `sdk/ios/.../EventEnvelope.swift`). */
data class IdentityRef(
    val namespace: String,
    val value: String,
    val primary: Boolean? = null,
    val source: String,
    val confidence: Double? = null,
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("namespace", namespace)
        put("value", value)
        primary?.let { put("primary", it) }
        put("source", source)
        confidence?.let { put("confidence", it) }
    }
}

/** Mirrors `mb_core::EventSource`. */
data class EventSource(val type: String, val name: String) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("type", type)
        put("name", name)
    }
}

/** Mirrors `mb_core::SchemaRef`. */
data class SchemaRef(val name: String, val version: String) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("name", name)
        put("version", version)
    }
}

/**
 * Mirrors `mb_core::EventEnvelope` — the JSON body `POST /events` expects.
 * `data`/`context` are `org.json.JSONObject` directly (Android's built-in
 * JSON support) rather than a generic map + a serialization dependency —
 * callers building nested structures use `JSONObject`/`JSONArray` the same
 * way they would for any other Android networking code.
 */
data class EventEnvelope(
    val eventId: String,
    val eventType: String,
    val timestamp: String,
    val source: EventSource,
    val identity: List<IdentityRef>,
    val context: JSONObject? = null,
    val data: JSONObject? = null,
    val schema: SchemaRef,
    val tenantId: String,
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("event_id", eventId)
        put("event_type", eventType)
        put("timestamp", timestamp)
        put("source", source.toJson())
        put("identity", JSONArray(identity.map { it.toJson() }))
        context?.let { put("context", it) }
        data?.let { put("data", it) }
        put("schema", schema.toJson())
        put("tenant_id", tenantId)
    }

    companion object {
        fun fromJson(json: JSONObject): EventEnvelope {
            val identityArray = json.getJSONArray("identity")
            val identity = (0 until identityArray.length()).map { i ->
                val obj = identityArray.getJSONObject(i)
                IdentityRef(
                    namespace = obj.getString("namespace"),
                    value = obj.getString("value"),
                    primary = if (obj.has("primary")) obj.getBoolean("primary") else null,
                    source = obj.getString("source"),
                    confidence = if (obj.has("confidence")) obj.getDouble("confidence") else null,
                )
            }
            val sourceObj = json.getJSONObject("source")
            val schemaObj = json.getJSONObject("schema")
            return EventEnvelope(
                eventId = json.getString("event_id"),
                eventType = json.getString("event_type"),
                timestamp = json.getString("timestamp"),
                source = EventSource(sourceObj.getString("type"), sourceObj.getString("name")),
                identity = identity,
                context = json.optJSONObject("context"),
                data = json.optJSONObject("data"),
                schema = SchemaRef(schemaObj.getString("name"), schemaObj.getString("version")),
                tenantId = json.getString("tenant_id"),
            )
        }
    }
}
