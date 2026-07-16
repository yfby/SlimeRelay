package com.github.yfby.slimerelay

object SlimeRelayConstants {
    const val SAMPLE_RATE = 16000
    const val CHANNELS = 1
    const val CHUNK_SAMPLES = 512
    const val SERVER_PORT = 34254
    const val DISCOVERY_PORT = 34255

    const val PROTOCOL_VERSION: Byte = 0x01
    const val MSG_DISCOVERY: Byte = 0x01
    const val MSG_HELLO: Byte = 0x02
    const val MSG_READY: Byte = 0x03
    const val MSG_RTP: Byte = 0x80.toByte()
    const val MSG_BYE: Byte = 0xC0.toByte()

    const val RTP_PT_PCM: Byte = 96
    const val RTP_HEADER_SIZE = 12
    const val SESSION_ID_SIZE = 16
    const val SERVER_NAME_SIZE = 32

    const val KEEPALIVE_INTERVAL_MS = 2000L
    const val KEEPALIVE_TIMEOUT_MS = 6000L

    const val NOTIFICATION_CHANNEL_ID = "slimerelay_service"
    const val NOTIFICATION_ID = 1
}
