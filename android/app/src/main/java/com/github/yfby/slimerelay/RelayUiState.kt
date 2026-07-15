package com.github.yfby.slimerelay

enum class RelayMode { SERVER, CLIENT }

enum class ConnectionState { DISCONNECTED, CONNECTING, CONNECTED }

data class RelayUiState(
    val mode: RelayMode = RelayMode.CLIENT,
    val state: ConnectionState = ConnectionState.DISCONNECTED,
    val serverIp: String = "127.0.0.1",
    val serverPort: String = SlimeRelayConstants.SERVER_PORT.toString(),
    val audioLevel: Float = 0f,
    val statusMessage: String = "",
    val isServiceBound: Boolean = false
)
