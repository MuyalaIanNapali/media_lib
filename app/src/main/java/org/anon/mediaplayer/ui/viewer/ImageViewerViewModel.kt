package org.anon.mediaplayer.ui.viewer

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.usecase.DecryptImageUseCase
import org.anon.mediaplayer.domain.usecase.mediaError
import org.anon.mediaplayer.ui.navigation.Routes

sealed interface ImageViewerUiState {
    data object Loading : ImageViewerUiState
    class Loaded(val image: ByteArray) : ImageViewerUiState
    data class Error(val error: MediaError) : ImageViewerUiState
}

@HiltViewModel
class ImageViewerViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    decryptImage: DecryptImageUseCase,
) : ViewModel() {

    private val mediaId: Long = checkNotNull(savedStateHandle[Routes.ARG_MEDIA_ID])

    private val _state = MutableStateFlow<ImageViewerUiState>(ImageViewerUiState.Loading)
    val state: StateFlow<ImageViewerUiState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            _state.value = decryptImage(mediaId).fold(
                onSuccess = { ImageViewerUiState.Loaded(it) },
                onFailure = { ImageViewerUiState.Error(it.mediaError) },
            )
        }
    }

    override fun onCleared() {
        // Drop the only reference to the decrypted bytes.
        _state.value = ImageViewerUiState.Loading
    }
}
