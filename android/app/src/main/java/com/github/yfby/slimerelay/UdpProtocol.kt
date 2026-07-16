package com.github.yfby.slimerelay

import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetSocketAddress

sealed class Message {
    data class Discovery(
        val serverName: String,
        val port: Int
    ) : Message()

    object Hello : Message()
    data class Ready(val sessionId: ByteArray) : Message()
    data class Rtp(val sequence: Int, val timestamp: Long, val ssrc: Long, val payload: ByteArray) : Message()
    data class Bye(val reason: String) : Message()
}

object UdpProtocol {

    fun buildHello(): ByteArray {
        return byteArrayOf(SlimeRelayConstants.PROTOCOL_VERSION, SlimeRelayConstants.MSG_HELLO)
    }

    fun buildReady(sessionId: ByteArray): ByteArray {
        val buf = ByteArray(1 + 1 + SlimeRelayConstants.SESSION_ID_SIZE)
        buf[0] = SlimeRelayConstants.PROTOCOL_VERSION
        buf[1] = SlimeRelayConstants.MSG_READY
        System.arraycopy(sessionId, 0, buf, 2, SlimeRelayConstants.SESSION_ID_SIZE)
        return buf
    }

    fun buildDiscovery(serverName: String, port: Int): ByteArray {
        val nameBytes = ByteArray(SlimeRelayConstants.SERVER_NAME_SIZE)
        val nameChars = serverName.toByteArray()
        System.arraycopy(nameChars, 0, nameBytes, 0, minOf(nameChars.size, SlimeRelayConstants.SERVER_NAME_SIZE))

        val buf = ByteArray(1 + 1 + SlimeRelayConstants.SERVER_NAME_SIZE + 2)
        buf[0] = SlimeRelayConstants.PROTOCOL_VERSION
        buf[1] = SlimeRelayConstants.MSG_DISCOVERY
        System.arraycopy(nameBytes, 0, buf, 2, SlimeRelayConstants.SERVER_NAME_SIZE)
        buf[2 + SlimeRelayConstants.SERVER_NAME_SIZE] = (port and 0xFF).toByte()
        buf[3 + SlimeRelayConstants.SERVER_NAME_SIZE] = (port shr 8 and 0xFF).toByte()
        return buf
    }

    fun buildRtpPacket(sequence: Int, timestamp: Long, ssrc: Long, payload: ByteArray): ByteArray {
        val buf = ByteArray(SlimeRelayConstants.RTP_HEADER_SIZE + payload.size)
        buf[0] = 0x80.toByte()
        buf[1] = SlimeRelayConstants.RTP_PT_PCM
        buf[2] = (sequence shr 8 and 0xFF).toByte()
        buf[3] = (sequence and 0xFF).toByte()
        buf[4] = (timestamp shr 24 and 0xFF).toByte()
        buf[5] = (timestamp shr 16 and 0xFF).toByte()
        buf[6] = (timestamp shr 8 and 0xFF).toByte()
        buf[7] = (timestamp and 0xFF).toByte()
        buf[8] = (ssrc shr 24 and 0xFF).toByte()
        buf[9] = (ssrc shr 16 and 0xFF).toByte()
        buf[10] = (ssrc shr 8 and 0xFF).toByte()
        buf[11] = (ssrc and 0xFF).toByte()
        System.arraycopy(payload, 0, buf, SlimeRelayConstants.RTP_HEADER_SIZE, payload.size)
        return buf
    }

    fun buildBye(reason: String): ByteArray {
        val reasonBytes = ByteArray(32)
        val chars = reason.toByteArray()
        System.arraycopy(chars, 0, reasonBytes, 0, minOf(chars.size, 32))

        val buf = ByteArray(1 + 1 + 32)
        buf[0] = SlimeRelayConstants.PROTOCOL_VERSION
        buf[1] = SlimeRelayConstants.MSG_BYE
        System.arraycopy(reasonBytes, 0, buf, 2, 32)
        return buf
    }

    fun parseMessage(data: ByteArray): Message? {
        if (data.size < 2) return null

        if (data[0] == 0x80.toByte() && data.size >= SlimeRelayConstants.RTP_HEADER_SIZE) {
            val sequence = (data[2].toInt() and 0xFF shl 8) or (data[3].toInt() and 0xFF)
            val timestamp = (data[4].toLong() and 0xFF shl 24) or
                    (data[5].toLong() and 0xFF shl 16) or
                    (data[6].toLong() and 0xFF shl 8) or
                    (data[7].toLong() and 0xFF)
            val ssrc = (data[8].toLong() and 0xFF shl 24) or
                    (data[9].toLong() and 0xFF shl 16) or
                    (data[10].toLong() and 0xFF shl 8) or
                    (data[11].toLong() and 0xFF)
            val payload = data.copyOfRange(SlimeRelayConstants.RTP_HEADER_SIZE, data.size)
            return Message.Rtp(sequence, timestamp, ssrc, payload)
        }

        if (data[0] != SlimeRelayConstants.PROTOCOL_VERSION) return null

        return when (data[1]) {
            SlimeRelayConstants.MSG_DISCOVERY -> {
                if (data.size < 1 + 1 + SlimeRelayConstants.SERVER_NAME_SIZE + 2) return null
                val nameBytes = data.copyOfRange(2, 2 + SlimeRelayConstants.SERVER_NAME_SIZE)
                val name = String(nameBytes).trimEnd('\u0000')
                val port = (data[2 + SlimeRelayConstants.SERVER_NAME_SIZE].toInt() and 0xFF) or
                        (data[3 + SlimeRelayConstants.SERVER_NAME_SIZE].toInt() and 0xFF shl 8)
                Message.Discovery(name, port)
            }
            SlimeRelayConstants.MSG_HELLO -> Message.Hello
            SlimeRelayConstants.MSG_READY -> {
                if (data.size < 1 + 1 + SlimeRelayConstants.SESSION_ID_SIZE) return null
                val sessionId = data.copyOfRange(2, 2 + SlimeRelayConstants.SESSION_ID_SIZE)
                Message.Ready(sessionId)
            }
            SlimeRelayConstants.MSG_RTP -> {
                null
            }
            SlimeRelayConstants.MSG_BYE -> {
                if (data.size < 1 + 1 + 32) return null
                val reasonBytes = data.copyOfRange(2, 2 + 32)
                val reason = String(reasonBytes).trimEnd('\u0000')
                Message.Bye(reason)
            }
            else -> null
        }
    }

    fun sendHello(socket: DatagramSocket, serverAddr: InetSocketAddress) {
        val msg = buildHello()
        val packet = DatagramPacket(msg, msg.size, serverAddr)
        socket.send(packet)
    }

    fun waitForHello(socket: DatagramSocket): InetSocketAddress {
        val buf = ByteArray(128)
        val packet = DatagramPacket(buf, buf.size)
        while (true) {
            socket.receive(packet)
            val data = buf.copyOf(packet.length)
            when (val msg = parseMessage(data)) {
                is Message.Hello -> return InetSocketAddress(packet.address, packet.port)
                else -> continue
            }
        }
    }

    fun sendReady(socket: DatagramSocket, clientAddr: InetSocketAddress, sessionId: ByteArray) {
        val msg = buildReady(sessionId)
        val packet = DatagramPacket(msg, msg.size, clientAddr)
        socket.send(packet)
    }

    fun waitForReady(socket: DatagramSocket): ByteArray {
        val buf = ByteArray(128)
        val packet = DatagramPacket(buf, buf.size)
        socket.receive(packet)
        val data = buf.copyOf(packet.length)
        val msg = parseMessage(data)
        if (msg !is Message.Ready) {
            throw IllegalStateException("Server did not respond with READY")
        }
        return msg.sessionId
    }

    fun waitForDiscovery(socket: DatagramSocket): Pair<String, InetSocketAddress> {
        val buf = ByteArray(128)
        val packet = DatagramPacket(buf, buf.size)
        while (true) {
            socket.receive(packet)
            val data = buf.copyOf(packet.length)
            when (val msg = parseMessage(data)) {
                is Message.Discovery -> {
                    val serverAddr = InetSocketAddress(packet.address, msg.port)
                    return Pair(msg.serverName, serverAddr)
                }
                else -> continue
            }
        }
    }

    fun f32ToBytes(samples: FloatArray): ByteArray {
        val buf = ByteArray(samples.size * 4)
        samples.forEachIndexed { i, s ->
            val bits = s.toRawBits()
            buf[i * 4] = (bits and 0xFF).toByte()
            buf[i * 4 + 1] = (bits shr 8 and 0xFF).toByte()
            buf[i * 4 + 2] = (bits shr 16 and 0xFF).toByte()
            buf[i * 4 + 3] = (bits shr 24 and 0xFF).toByte()
        }
        return buf
    }

    fun bytesToF32(bytes: ByteArray): FloatArray {
        val count = bytes.size / 4
        return FloatArray(count) { i ->
            val bits = (bytes[i * 4].toInt() and 0xFF) or
                    (bytes[i * 4 + 1].toInt() and 0xFF shl 8) or
                    (bytes[i * 4 + 2].toInt() and 0xFF shl 16) or
                    (bytes[i * 4 + 3].toInt() and 0xFF shl 24)
            Float.fromBits(bits)
        }
    }
}
