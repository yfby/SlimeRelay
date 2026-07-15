package com.github.yfby.slimerelay

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
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

class RelayService : Service() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val binder = LocalBinder()

    private val _uiState = MutableStateFlow(RelayUiState())
    val uiState: StateFlow<RelayUiState> = _uiState.asStateFlow()

    private var socket: DatagramSocket? = null
    private var recvJob: kotlinx.coroutines.Job? = null

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

    fun connect() {
        val state = _uiState.value
        if (state.state == ConnectionState.CONNECTED || state.state == ConnectionState.CONNECTING) return

        _uiState.value = state.copy(state = ConnectionState.CONNECTING, statusMessage = "Connecting...")

        val notification = buildNotification("Relay ${state.mode.name.lowercase()} mode active")
        startForeground(SlimeRelayConstants.NOTIFICATION_ID, notification)

        when (state.mode) {
            RelayMode.CLIENT -> startClient(state.serverIp, state.serverPort.toIntOrNull() ?: SlimeRelayConstants.SERVER_PORT)
            RelayMode.SERVER -> startServer(state.serverPort.toIntOrNull() ?: SlimeRelayConstants.SERVER_PORT)
        }
    }

    fun disconnect() {
        recvJob?.cancel()
        recvJob = null
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

    private fun startClient(ip: String, port: Int) {
        recvJob = scope.launch {
            try {
                val sock = DatagramSocket()
                socket = sock

                val serverAddr = InetSocketAddress(ip, port)
                UdpProtocol.sendHello(sock, serverAddr)
                updateStatus("Sent HELLO, waiting for READY...")

                sock.soTimeout = 5000
                UdpProtocol.waitForReady(sock)
                sock.soTimeout = 100

                updateStatus("Connected! Receiving audio...")
                _uiState.value = _uiState.value.copy(state = ConnectionState.CONNECTED)

                val player = SpeakerPlayer { e -> updateStatus("Audio error: $e") }
                player.start()

                val recvBuf = ByteArray(SlimeRelayConstants.CHUNK_SAMPLES * 4)
                try {
                    while (true) {
                        try {
                            val packet = DatagramPacket(recvBuf, recvBuf.size)
                            sock.receive(packet)
                            val samples = UdpProtocol.bytesToF32(recvBuf.copyOf(packet.length))
                            player.writeSamples(samples)
                            _uiState.value = _uiState.value.copy(audioLevel = player.computeLevel(samples))
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

    private fun startServer(port: Int) {
        recvJob = scope.launch {
            try {
                val sock = DatagramSocket(port)
                socket = sock
                updateStatus("Listening on port $port, waiting for client...")

                val clientAddr = UdpProtocol.waitForHello(sock)
                UdpProtocol.sendReady(sock, clientAddr)

                updateStatus("Client connected! Streaming audio...")
                _uiState.value = _uiState.value.copy(state = ConnectionState.CONNECTED)

                val recorder = MicRecorder(
                    onSamples = { samples ->
                        try {
                            val bytes = UdpProtocol.f32ToBytes(samples)
                            val packet = DatagramPacket(bytes, bytes.size, clientAddr)
                            sock.send(packet)
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
