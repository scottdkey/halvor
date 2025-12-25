package dev.scottkey.halvor

import com.sun.jna.Library
import com.sun.jna.Native

/** Kotlin wrapper for Halvor Rust FFI Auto-generated bindings are in GeneratedBindings.kt */
class HalvorClient(private val agentPort: UShort? = null) {
    private val nativeLib: HalvorNativeLib = Native.load("halvor_ffi", HalvorNativeLib::class.java)

    interface HalvorNativeLib : Library {
        fun halvor_client_new(agentPort: UShort): Long
        fun halvor_client_free(ptr: Long)
        fun halvor_client_discover_agents(ptr: Long): String?
        fun halvor_string_free(ptr: String)
    }

    private val clientPtr: Long = nativeLib.halvor_client_new(agentPort ?: 0u)

    fun discoverAgents(): List<DiscoveredHost> {
        val json =
                nativeLib.halvor_client_discover_agents(clientPtr)
                        ?: throw HalvorException("Failed to discover agents")

        // Parse JSON to List<DiscoveredHost>
        // Implementation depends on JSON library
        return emptyList() // Placeholder
    }

    protected fun finalize() {
        nativeLib.halvor_client_free(clientPtr)
    }
}

class HalvorException(message: String) : Exception(message)
