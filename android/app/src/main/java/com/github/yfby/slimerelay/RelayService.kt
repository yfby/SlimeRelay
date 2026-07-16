package com.github.yfby.slimerelay

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetSocketAddress
import java.util.UUID

class RelayService : Service() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val binder = LocalBinder()

    private val _uiState = MutableStateFlow(RelayUiState())
    val uiState: StateFlow<RelayUiState> = _uiState.asStateFlow()

    private var socket: DatagramSocket? = null
    private var recvJob: kotlinx.coroutines.Job? = null
    private var discoveryJob: kotlinx.coroutines.Job? = null

    inner class LocalBinder : Binder() {
        fun getService(): RelayService = this@RelayService
    }

    override fun onBind(intent: Intent): IBinder = binder

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return START_STICKY
    }

    override fun onDestroy() {
        disconnect()
        scope.cancel()
        super.onDestroy()
    }

    fun updateMode(mode: RelayMode) {
        _uiState.value = _uiState.value.copy(mode = mode)
    }

    fun updateServerIp(ip: String) {
        _uiState.value = _uiState.value.copy(serverIp = ip)
    }

    fun updateServerPort(port: String) {
        _uiState.value = _uiState.value.copy(serverPort = port)
    }

    fun updateServerName(name: String) {
        _uiState.value = _uiState.value.copy(serverName = name)
    }

    fun updateDiscoverMode(discover: Boolean) {
        _uiState.value = _uiState.value.copy(discoverMode = discover)
    }

    fun connect() {
        val state = _uiState.value
        if (state.state == ConnectionState.CONNECTED || state.state == ConnectionState.CONNECTING) return

        _uiState.value = state.copy(state = ConnectionState.CONNECTING, statusMessage = "Connecting...")

        val notification = buildNotification("Relay ${state.mode.name.lowercase()} mode active")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(SlimeRelayConstants.NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE)
        } else {
            startForeground(SlimeRelayConstants.NOTIFICATION_ID, notification)
        }

        when (state.mode) {
            RelayMode.CLIENT -> startClient(state)
            RelayMode.SERVER -> startServer(state)
        }
    }

    fun disconnect() {
        recvJob?.cancel()
        recvJob = null
        discoveryJob?.cancel()
        discoveryJob = null
        socket?.close()
        socket = null
        _uiState.value = _uiState.value.copy(
            state = ConnectionState.DISCONNECTED,
            statusMessage = "",
            audioLevel = 0f
        )
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun startClient(state: RelayUiState) {
        if (state.discoverMode) {
            discoveryJob = scope.launch {
                try {
                    updateStatus("Searching for server...")
                    val discoverySocket = DatagramSocket(SlimeRelayConstants.DISCOVERY_PORT)
                    discoverySocket.soTimeout = 30000

                    val (serverName, serverAddr) = UdpProtocol.waitForDiscovery(discoverySocket)
                    discoverySocket.close()

                    updateStatus("Discovered server '$serverName' at $serverAddr")
                    connectToServer(serverAddr)
                } catch (e: Exception) {
                    if (recvJob?.isCancelled != true && discoveryJob?.isCancelled != true) {
                        updateStatus("Discovery failed: ${e.message}")
                        _uiState.value = _uiState.value.copy(state = ConnectionState.DISCONNECTED)
                    }
                }
            }
        } else {
            val serverAddr = InetSocketAddress(
                state.serverIp,
                state.serverPort.toIntOrNull() ?: SlimeRelayConstants.SERVER_PORT
            )
            connectToServer(serverAddr)
        }
    }

    private fun connectToServer(serverAddr: InetSocketAddress) {
        recvJob = scope.launch {
            try {
                val sock = DatagramSocket()
                socket = sock

                UdpProtocol.sendHello(sock, serverAddr)
                updateStatus("Sent HELLO, waiting for READY...")

                sock.soTimeout = 5000
                val sessionId = UdpProtocol.waitForReady(sock)
                sock.soTimeout = 100

                updateStatus("Connected! Receiving audio...")
                _uiState.value = _uiState.value.copy(state = ConnectionState.CONNECTED)

                val player = SpeakerPlayer { e -> updateStatus("Audio error: $e") }
                player.start()

                val recvBuf = ByteArray(SlimeRelayConstants.CHUNK_SAMPLES * 4 + SlimeRelayConstants.RTP_HEADER_SIZE)
                var lastRtpReceived = System.currentTimeMillis()

                try {
                    while (true) {
                        if (System.currentTimeMillis() - lastRtpReceived > SlimeRelayConstants.KEEPALIVE_TIMEOUT_MS) {
                            updateStatus("Connection lost: no packets received")
                            break
                        }

                        try {
                            val packet = DatagramPacket(recvBuf, recvBuf.size)
                            sock.receive(packet)
                            val msg = UdpProtocol.parseMessage(recvBuf.copyOf(packet.length))
                            if (msg is Message.Rtp) {
                                lastRtpReceived = System.currentTimeMillis()
                                val samples = UdpProtocol.bytesToF32(msg.payload)
                                player.writeSamples(samples)
                                _uiState.value = _uiState.value.copy(audioLevel = player.computeLevel(samples))
                            } else if (msg is Message.Bye) {
                                updateStatus("Server sent BYE: ${msg.reason}")
                                break
                            }
                        } catch (e: java.net.SocketTimeoutException) {
                            continue
                        }
                    }
                } finally {
                    player.stop()
                }
            } catch (e: Exception) {
                if (recvJob?.isCancelled != true) {
                    updateStatus("Error: ${e.message}")
                    _uiState.value = _uiState.value.copy(state = ConnectionState.DISCONNECTED)
                }
            }
        }
    }

    private fun startServer(state: RelayUiState) {
        recvJob = scope.launch {
            try {
                val sock = DatagramSocket(SlimeRelayConstants.SERVER_PORT)
                socket = sock

                discoveryJob = scope.launch {
                    try {
                        val discoverySocket = DatagramSocket()
                        discoverySocket.broadcast = true
                        val serverName = state.serverName.ifBlank { android.os.Build.MODEL }
                        val discoveryPacket = UdpProtocol.buildDiscovery(serverName, SlimeRelayConstants.SERVER_PORT)
                        val broadcastAddr = InetSocketAddress("255.255.255.255", SlimeRelayConstants.DISCOVERY_PORT)

                        while (true) {
                            try {
                                val packet = DatagramPacket(discoveryPacket, discoveryPacket.size, broadcastAddr)
                                discoverySocket.send(packet)
                            } catch (e: Exception) {
                                Log.w("RelayService", "Discovery broadcast error: ${e.message}")
                            }
                            kotlinx.coroutines.delay(SlimeRelayConstants.KEEPALIVE_INTERVAL_MS)
                        }
                    } catch (e: Exception) {
                        Log.e("RelayService", "Discovery broadcast failed: ${e.message}")
                    }
                }

                updateStatus("Listening on port ${SlimeRelayConstants.SERVER_PORT}, waiting for client...")

                val clientAddr = UdpProtocol.waitForHello(sock)

                val sessionId = UUID.randomUUID().toString().replace("-", "").toByteArray().copyOf(16)
                UdpProtocol.sendReady(sock, clientAddr, sessionId)

                updateStatus("Client connected! Streaming audio...")
                _uiState.value = _uiState.value.copy(state = ConnectionState.CONNECTED)

                val ssrc = (Math.random() * 0xFFFFFFFFL).toLong() and 0xFFFFFFFFL
                var sequence = 0
                var timestamp = 0L

                val recorder = MicRecorder(
                    onSamples = { samples ->
                        try {
                            val bytes = UdpProtocol.f32ToBytes(samples)
                            val rtpPacket = UdpProtocol.buildRtpPacket(sequence, timestamp, ssrc, bytes)
                            val packet = DatagramPacket(rtpPacket, rtpPacket.size, clientAddr)
                            sock.send(packet)
                            sequence = (sequence + 1) and 0xFFFF
                            timestamp = (timestamp + SlimeRelayConstants.CHUNK_SAMPLES) and 0xFFFFFFFFL
                            _uiState.value = _uiState.value.copy(audioLevel = MicRecorderHelper.computeLevel(samples))
                        } catch (e: Exception) {
                            if (recvJob?.isCancelled != true) {
                                updateStatus("Send error: ${e.message}")
                            }
                        }
                    },
                    onError = { e -> updateStatus("Audio error: $e") }
                )
                recorder.start()

                try {
                    while (true) {
                        kotlinx.coroutines.delay(100)
                    }
                } finally {
                    recorder.stop()
                }
            } catch (e: Exception) {
                if (recvJob?.isCancelled != true) {
                    updateStatus("Error: ${e.message}")
                    _uiState.value = _uiState.value.copy(state = ConnectionState.DISCONNECTED)
                }
            }
        }
    }

    private fun updateStatus(msg: String) {
        _uiState.value = _uiState.value.copy(statusMessage = msg)
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            SlimeRelayConstants.NOTIFICATION_CHANNEL_ID,
            "SlimeRelay Service",
            NotificationManager.IMPORTANCE_LOW
        )
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(text: String): Notification {
        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent, PendingIntent.FLAG_IMMUTABLE
        )

        return Notification.Builder(this, SlimeRelayConstants.NOTIFICATION_CHANNEL_ID)
            .setContentTitle("SlimeRelay")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }
}

private object MicRecorderHelper {
    fun computeLevel(samples: FloatArray): Float {
        if (samples.isEmpty()) return 0f
        val sum = samples.sumOf { kotlin.math.abs(it).toDouble() }
        return (sum / samples.size).coerceIn(0.0, 1.0).toFloat()
    }
}
