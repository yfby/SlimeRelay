package com.github.yfby.slimerelay

import android.Manifest
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.Bundle
import android.os.IBinder
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.github.yfby.slimerelay.ui.theme.SlimeRelayTheme

class MainActivity : ComponentActivity() {

    private var relayService: RelayService? = null
    private var bound = false

    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            val binder = service as RelayService.LocalBinder
            relayService = binder.getService()
            bound = true
        }

        override fun onServiceDisconnected(name: ComponentName) {
            relayService = null
            bound = false
        }
    }

    private val requestPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            relayService?.connect()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        Intent(this, RelayService::class.java).also { intent ->
            startService(intent)
            bindService(intent, connection, Context.BIND_AUTO_CREATE)
        }

        setContent {
            SlimeRelayTheme {
                val state by (relayService?.uiState?.collectAsState() ?: throw IllegalStateException("Service not bound"))

                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    RelayScreen(
                        state = state,
                        onModeChange = { relayService?.updateMode(it) },
                        onServerIpChange = { relayService?.updateServerIp(it) },
                        onServerPortChange = { relayService?.updateServerPort(it) },
                        onConnectToggle = {
                            if (state.state == ConnectionState.CONNECTED) {
                                relayService?.disconnect()
                            } else {
                                checkAndConnect()
                            }
                        },
                        modifier = Modifier.padding(innerPadding)
                    )
                }
            }
        }
    }

    override fun onDestroy() {
        if (bound) {
            unbindService(connection)
            bound = false
        }
        super.onDestroy()
    }

    private fun checkAndConnect() {
        val hasPermission = ContextCompat.checkSelfPermission(
            this, Manifest.permission.RECORD_AUDIO
        ) == PackageManager.PERMISSION_GRANTED

        if (hasPermission) {
            relayService?.connect()
        } else {
            requestPermissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
        }
    }
}

@Composable
fun RelayScreen(
    state: RelayUiState,
    onModeChange: (RelayMode) -> Unit,
    onServerIpChange: (String) -> Unit,
    onServerPortChange: (String) -> Unit,
    onConnectToggle: () -> Unit,
    modifier: Modifier = Modifier
) {
    val isConnecting = state.state == ConnectionState.CONNECTING
    val isConnected = state.state == ConnectionState.CONNECTED
    val animatedLevel by animateFloatAsState(targetValue = state.audioLevel, label = "audioLevel")

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        Text(
            text = "SlimeRelay",
            style = MaterialTheme.typography.headlineLarge
        )

        SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
            SegmentedButton(
                selected = state.mode == RelayMode.CLIENT,
                onClick = { onModeChange(RelayMode.CLIENT) },
                enabled = !isConnected && !isConnecting,
                shape = SegmentedButtonDefaults.itemShape(index = 0, count = 2)
            ) {
                Text("Client")
            }
            SegmentedButton(
                selected = state.mode == RelayMode.SERVER,
                onClick = { onModeChange(RelayMode.SERVER) },
                enabled = !isConnected && !isConnecting,
                shape = SegmentedButtonDefaults.itemShape(index = 1, count = 2)
            ) {
                Text("Server")
            }
        }

        if (state.mode == RelayMode.CLIENT) {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant
                )
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    OutlinedTextField(
                        value = state.serverIp,
                        onValueChange = onServerIpChange,
                        label = { Text("Server IP") },
                        singleLine = true,
                        enabled = !isConnected && !isConnecting,
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = state.serverPort,
                        onValueChange = onServerPortChange,
                        label = { Text("Port") },
                        singleLine = true,
                        enabled = !isConnected && !isConnecting,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        } else {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant
                )
            ) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    OutlinedTextField(
                        value = state.serverPort,
                        onValueChange = onServerPortChange,
                        label = { Text("Listen Port") },
                        singleLine = true,
                        enabled = !isConnected && !isConnecting,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        }

        if (state.statusMessage.isNotEmpty()) {
            Text(
                text = state.statusMessage,
                style = MaterialTheme.typography.bodyMedium,
                color = when (state.state) {
                    ConnectionState.CONNECTED -> MaterialTheme.colorScheme.primary
                    ConnectionState.CONNECTING -> MaterialTheme.colorScheme.tertiary
                    else -> MaterialTheme.colorScheme.error
                }
            )
        }

        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(
                text = "Audio Level",
                style = MaterialTheme.typography.labelMedium
            )
            LinearProgressIndicator(
                progress = { animatedLevel },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(8.dp),
            )
        }

        Spacer(modifier = Modifier.weight(1f))

        Button(
            onClick = onConnectToggle,
            enabled = !isConnecting,
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp),
            colors = if (isConnected) {
                ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error)
            } else {
                ButtonDefaults.buttonColors()
            }
        ) {
            Text(
                text = when {
                    isConnecting -> "Connecting..."
                    isConnected -> "Disconnect"
                    else -> "Connect"
                },
                style = MaterialTheme.typography.titleMedium
            )
        }
    }
}

@Preview(showBackground = true)
@Composable
fun RelayScreenPreview() {
    SlimeRelayTheme {
        RelayScreen(
            state = RelayUiState(
                mode = RelayMode.CLIENT,
                state = ConnectionState.DISCONNECTED,
                serverIp = "192.168.1.100",
                audioLevel = 0.3f,
                statusMessage = "Connected! Receiving audio..."
            ),
            onModeChange = {},
            onServerIpChange = {},
            onServerPortChange = {},
            onConnectToggle = {}
        )
    }
}
