package org.anon.mediaplayer.ui.securefolder

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.SecureMedia
import org.anon.mediaplayer.domain.usecase.GetThumbnailUseCase
import org.anon.mediaplayer.domain.usecase.ObserveSecureMediaUseCase
import org.anon.mediaplayer.domain.usecase.mediaError

sealed interface SecureFolderUiState {
    data object Loading : SecureFolderUiState
    data class Content(val items: List<SecureMedia>) : SecureFolderUiState
}

sealed interface ThumbnailState {
    /** Identity equality is intended: the bytes are never compared. */
    class Ready(val jpeg: ByteArray) : ThumbnailState
    data class Failed(val error: MediaError) : ThumbnailState
}

@HiltViewModel
class SecureFolderViewModel @Inject constructor(
    observeSecureMedia: ObserveSecureMediaUseCase,
    private val getThumbnail: GetThumbnailUseCase,
) : ViewModel() {

    val state: StateFlow<SecureFolderUiState> = observeSecureMedia()
        .map { SecureFolderUiState.Content(it) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), SecureFolderUiState.Loading)

    private val _thumbnails = MutableStateFlow<Map<Long, ThumbnailState>>(emptyMap())
    val thumbnails: StateFlow<Map<Long, ThumbnailState>> = _thumbnails.asStateFlow()

    private val requested = mutableSetOf<Long>()

    /** Called as each tile appears; each thumbnail is generated at most once. */
    fun loadThumbnail(mediaId: Long) {
        if (!requested.add(mediaId)) return
        viewModelScope.launch {
            val thumbnail = getThumbnail(mediaId).fold(
                onSuccess = { ThumbnailState.Ready(it) },
                onFailure = { ThumbnailState.Failed(it.mediaError) },
            )
            _thumbnails.update { it + (mediaId to thumbnail) }
        }
    }
}
