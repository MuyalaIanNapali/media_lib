package org.anon.mediaplayer.ui.viewer

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.usecase.GetThumbnailUseCase
import org.anon.mediaplayer.domain.usecase.mediaError
import org.anon.mediaplayer.playback.MediaPlayerFactory
import org.anon.mediaplayer.playback.toMediaError
import org.anon.mediaplayer.ui.navigation.Routes

data class MediaPlayerUiState(
    val isAudio: Boolean = false,
    /** Video preview frame decrypted from a few chunks, shown until Play is pressed. */
    val preview: ByteArray? = null,
    val player: Player? = null,
    val isPreparing: Boolean = false,
    val error: MediaError? = null,
) {
    // Identity on the byte array is enough for UI state diffing.
    override fun equals(other: Any?) = other is MediaPlayerUiState &&
        isAudio == other.isAudio && preview === other.preview && player === other.player &&
        isPreparing == other.isPreparing && error == other.error

    override fun hashCode() = System.identityHashCode(preview) * 31 + (player?.hashCode() ?: 0)
}

@HiltViewModel
class MediaPlayerViewModel @Inject constructor(
    savedStateHandle: SavedStateHandle,
    getThumbnail: GetThumbnailUseCase,
    private val playerFactory: MediaPlayerFactory,
) : ViewModel() {

    private val mediaId: Long = checkNotNull(savedStateHandle[Routes.ARG_MEDIA_ID])
    private val isAudio: Boolean = savedStateHandle[Routes.ARG_IS_AUDIO] ?: false

    private val _state = MutableStateFlow(MediaPlayerUiState(isAudio = isAudio))
    val state: StateFlow<MediaPlayerUiState> = _state.asStateFlow()

    private val errorListener = object : Player.Listener {
        override fun onPlayerError(error: PlaybackException) {
            _state.update { it.copy(error = error.toMediaError()) }
        }
    }

    init {
        // Audio has no preview frame.
        if (!isAudio) viewModelScope.launch {
            // A missing preview isn't fatal; playback reports its own errors.
            getThumbnail(mediaId).onSuccess { preview -> _state.update { it.copy(preview = preview) } }
        }
    }

    fun play() {
        val current = _state.value
        if (current.player != null || current.isPreparing) return

        _state.update { it.copy(isPreparing = true, error = null) }
        viewModelScope.launch {
            try {
                val player = playerFactory.create(mediaId).apply {
                    addListener(errorListener)
                    playWhenReady = true
                }
                _state.update { it.copy(player = player, isPreparing = false) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _state.update { it.copy(isPreparing = false, error = e.mediaError) }
            }
        }
    }

    fun pause() {
        _state.value.player?.pause()
    }

    override fun onCleared() {
        _state.value.player?.run {
            removeListener(errorListener)
            release()
        }
        _state.value = MediaPlayerUiState(isAudio = isAudio)
    }
}
