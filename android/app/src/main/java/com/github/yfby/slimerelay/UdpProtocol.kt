package com.github.yfby.slimerelay

import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetSocketAddress

object UdpProtocol {

    fun sendHello(socket: DatagramSocket, serverAddr: InetSocketAddress) {
        val packet = DatagramPacket(
            SlimeRelayConstants.HELLO,
            SlimeRelayConstants.HELLO.size,
            serverAddr
        )
        socket.send(packet)
    }

    fun waitForHello(socket: DatagramSocket): InetSocketAddress {
        val buf = ByteArray(SlimeRelayConstants.HANDSHAKE_BUF_SIZE)
        val packet = DatagramPacket(buf, buf.size)
        socket.receive(packet)
        val data = buf.copyOf(packet.length)
        if (data.contentEquals(SlimeRelayConstants.HELLO).not()) {
            throw IllegalStateException("Expected HELLO handshake, got: ${String(data)}")
        }
        return InetSocketAddress(packet.address, packet.port)
    }

    fun sendReady(socket: DatagramSocket, clientAddr: InetSocketAddress) {
        val packet = DatagramPacket(
            SlimeRelayConstants.READY,
            SlimeRelayConstants.READY.size,
            clientAddr
        )
        socket.send(packet)
    }

    fun waitForReady(socket: DatagramSocket) {
        val buf = ByteArray(SlimeRelayConstants.HANDSHAKE_BUF_SIZE)
        val packet = DatagramPacket(buf, buf.size)
        socket.receive(packet)
        val data = buf.copyOf(packet.length)
        if (data.contentEquals(SlimeRelayConstants.READY).not()) {
            throw IllegalStateException("Server did not respond with READY, got: ${String(data)}")
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
