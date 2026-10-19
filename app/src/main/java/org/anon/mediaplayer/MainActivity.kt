package org.anon.mediaplayer

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import dagger.hilt.android.AndroidEntryPoint
import org.anon.mediaplayer.ui.navigation.AppNavHost
import org.anon.mediaplayer.ui.theme.MediaPlayerTheme

@AndroidEntryPoint
class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            MediaPlayerTheme {
                AppNavHost()
            }
        }
    }
}
