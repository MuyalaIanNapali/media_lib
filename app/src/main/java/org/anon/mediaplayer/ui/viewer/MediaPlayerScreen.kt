package org.anon.mediaplayer.ui.viewer

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.Player
import androidx.media3.ui.PlayerView
import coil.compose.AsyncImage
import org.anon.mediaplayer.R

@Composable
fun MediaPlayerRoute(
    onBack: () -> Unit,
    viewModel: MediaPlayerViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()

    LifecycleEventEffect(Lifecycle.Event.ON_STOP) { viewModel.pause() }

    ViewerFrame(onBack) {
        val player = state.player
        val error = state.error
        when {
            error != null -> ErrorMessage(error)
            player != null -> PlayerSurface(player, alwaysShowControls = state.isAudio)
            else -> PlaybackPreview(
                isAudio = state.isAudio,
                preview = state.preview,
                isPreparing = state.isPreparing,
                onPlay = viewModel::play,
            )
        }
    }
}

/** Before playback: the video's preview frame (or audio artwork) and Play. */
@Composable
private fun PlaybackPreview(isAudio: Boolean, preview: ByteArray?, isPreparing: Boolean, onPlay: () -> Unit) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        if (isAudio) {
            AudioArtwork(Modifier.align(Alignment.Center).padding(bottom = 160.dp))
        } else if (preview != null) {
            AsyncImage(
                model = preview,
                contentDescription = stringResource(R.string.viewer_video_preview),
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }
        if (isPreparing) {
            CircularProgressIndicator()
        } else {
            Button(onClick = onPlay) {
                Icon(painterResource(R.drawable.ic_play), contentDescription = null)
                Text(stringResource(R.string.viewer_play), Modifier.padding(start = 8.dp))
            }
        }
    }
}

@Composable
private fun AudioArtwork(modifier: Modifier = Modifier) {
    Icon(
        painter = painterResource(R.drawable.ic_music),
        contentDescription = stringResource(R.string.viewer_audio),
        tint = Color.White,
        modifier = modifier.size(120.dp),
    )
}

@Composable
private fun PlayerSurface(player: Player, alwaysShowControls: Boolean) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        AndroidView(
            factory = { context ->
                PlayerView(context).apply {
                    this.player = player
                    if (alwaysShowControls) {
                        // Audio has nothing to look at: keep the controls up.
                        controllerShowTimeoutMs = 0
                        controllerHideOnTouch = false
                        setShutterBackgroundColor(android.graphics.Color.TRANSPARENT)
                        showController()
                    }
                }
            },
            update = { it.player = player },
            onRelease = { it.player = null },
            modifier = Modifier.fillMaxSize(),
        )
        // Drawn above the player view (which paints its own background);
        // it has no pointer input, so touches still reach the controls.
        if (alwaysShowControls) AudioArtwork(Modifier.padding(bottom = 320.dp))
    }
}
