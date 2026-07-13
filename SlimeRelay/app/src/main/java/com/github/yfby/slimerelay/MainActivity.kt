package com.github.yfby.slimerelay

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import com.github.yfby.slimerelay.ui.theme.SlimeRelayTheme

/// Main Activity for the SlimeRelay Android app.
///
/// This is the entry point for the Android client. Currently displays a placeholder
/// "Hello Android!" screen. Future versions will implement:
/// - Audio capture/playback using Android's AudioRecord/AudioTrack APIs
/// - UDP networking to communicate with the desktop server/client
/// - UI for device selection and connection status
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Enable edge-to-edge display for modern Android UI styling.
        enableEdgeToEdge()
        setContent {
            SlimeRelayTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    Greeting(
                        name = "Android",
                        modifier = Modifier.padding(innerPadding)
                    )
                }
            }
        }
    }
}

/// Simple greeting composable - currently a placeholder.
@Composable
fun Greeting(name: String, modifier: Modifier = Modifier) {
    Text(
        text = "Hello $name!",
        modifier = modifier
    )
}

/// Preview function for Android Studio's design editor.
@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    SlimeRelayTheme {
        Greeting("Android")
    }
}
