package org.anon.mediaplayer.ui.securefolder

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil.compose.AsyncImage
import org.anon.mediaplayer.R
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.model.SecureMedia

@Composable
fun SecureFolderRoute(
    onOpenMedia: (SecureMedia) -> Unit,
    viewModel: SecureFolderViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val thumbnails by viewModel.thumbnails.collectAsStateWithLifecycle()

    SecureFolderScreen(
        state = state,
        thumbnails = thumbnails,
        onLoadThumbnail = viewModel::loadThumbnail,
        onOpenMedia = onOpenMedia,
    )
}

@Composable
fun SecureFolderScreen(
    state: SecureFolderUiState,
    thumbnails: Map<Long, ThumbnailState>,
    onLoadThumbnail: (Long) -> Unit,
    onOpenMedia: (SecureMedia) -> Unit,
    modifier: Modifier = Modifier,
) {
    when (state) {
        SecureFolderUiState.Loading -> Box(modifier.fillMaxSize(), Alignment.Center) {
            CircularProgressIndicator()
        }

        is SecureFolderUiState.Content -> if (state.items.isEmpty()) {
            Box(modifier.fillMaxSize().padding(32.dp), Alignment.Center) {
                Text(
                    text = stringResource(R.string.folder_empty),
                    style = MaterialTheme.typography.bodyLarge,
                    textAlign = TextAlign.Center,
                )
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Adaptive(110.dp),
                modifier = modifier.fillMaxSize(),
                contentPadding = PaddingValues(8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(state.items, key = { it.id }) { media ->
                    MediaTile(
                        media = media,
                        thumbnail = thumbnails[media.id],
                        onLoadThumbnail = { onLoadThumbnail(media.id) },
                        onClick = { onOpenMedia(media) },
                    )
                }
            }
        }
    }
}

@Composable
private fun MediaTile(
    media: SecureMedia,
    thumbnail: ThumbnailState?,
    onLoadThumbnail: () -> Unit,
    onClick: () -> Unit,
) {
    // Audio has no preview frame to decrypt.
    if (media.type != MediaType.AUDIO) {
        LaunchedEffect(media.id) { onLoadThumbnail() }
    }

    val description = stringResource(
        when (media.type) {
            MediaType.IMAGE -> R.string.folder_item_image
            MediaType.VIDEO -> R.string.folder_item_video
            MediaType.AUDIO -> R.string.folder_item_audio
        }
    )

    Box(
        modifier = Modifier
            .aspectRatio(1f)
            .clip(RoundedCornerShape(8.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        when {
            media.type == MediaType.AUDIO -> Icon(
                painter = painterResource(R.drawable.ic_music),
                contentDescription = description,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.size(48.dp),
            )

            thumbnail is ThumbnailState.Ready -> AsyncImage(
                model = thumbnail.jpeg,
                contentDescription = description,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )

            thumbnail is ThumbnailState.Failed -> Icon(
                painter = painterResource(R.drawable.ic_lock),
                contentDescription = description,
                tint = MaterialTheme.colorScheme.error,
            )

            else -> CircularProgressIndicator(Modifier.size(24.dp), strokeWidth = 2.dp)
        }

        if (media.type == MediaType.VIDEO) {
            Icon(
                painter = painterResource(R.drawable.ic_play),
                contentDescription = null,
                tint = Color.White,
                modifier = Modifier
                    .size(36.dp)
                    .background(Color.Black.copy(alpha = 0.5f), CircleShape)
                    .padding(6.dp),
            )
        }
    }
}
