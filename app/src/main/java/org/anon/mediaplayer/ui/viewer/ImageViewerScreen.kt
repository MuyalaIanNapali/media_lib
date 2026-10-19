package org.anon.mediaplayer.ui.viewer

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.stringResource
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil.compose.AsyncImage
import org.anon.mediaplayer.R

@Composable
fun ImageViewerRoute(
    onBack: () -> Unit,
    viewModel: ImageViewerViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()

    ViewerFrame(onBack) {
        when (val current = state) {
            ImageViewerUiState.Loading -> CircularProgressIndicator()
            is ImageViewerUiState.Error -> ErrorMessage(current.error)
            is ImageViewerUiState.Loaded -> AsyncImage(
                model = current.image,
                contentDescription = stringResource(R.string.viewer_image),
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}
