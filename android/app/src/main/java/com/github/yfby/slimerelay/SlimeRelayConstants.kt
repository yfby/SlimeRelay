package com.github.yfby.slimerelay

object SlimeRelayConstants {
    const val SAMPLE_RATE = 16000
    const val CHANNELS = 1
    const val CHUNK_SAMPLES = 512
    const val SERVER_PORT = 34254

    val HELLO = "HELLO".toByteArray()
    val READY = "READY".toByteArray()
    val HANDSHAKE_BUF_SIZE = 64

    const val NOTIFICATION_CHANNEL_ID = "slimerelay_service"
    const val NOTIFICATION_ID = 1
}
