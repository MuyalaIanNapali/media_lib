package org.anon.mediaplayer.ui.encrypt

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.usecase.ImportMediaUseCase
import org.anon.mediaplayer.domain.usecase.ResolveMediaTypeUseCase
import org.anon.mediaplayer.domain.usecase.mediaError

data class EncryptUiState(
    val selectedUri: String? = null,
    val mediaType: MediaType? = null,
    val isEncrypting: Boolean = false,
    val isEncrypted: Boolean = false,
    val error: MediaError? = null,
)

@HiltViewModel
class EncryptViewModel @Inject constructor(
    private val resolveMediaType: ResolveMediaTypeUseCase,
    private val importMedia: ImportMediaUseCase,
) : ViewModel() {

    private val _state = MutableStateFlow(EncryptUiState())
    val state: StateFlow<EncryptUiState> = _state.asStateFlow()

    fun onMediaPicked(uri: String?) {
        if (uri == null || _state.value.isEncrypting) return
        _state.value = EncryptUiState(selectedUri = uri, mediaType = resolveMediaType(uri))
    }

    fun encrypt() {
        val current = _state.value
        val uri = current.selectedUri ?: return
        if (current.isEncrypting) return

        _state.update { it.copy(isEncrypting = true, isEncrypted = false, error = null) }
        viewModelScope.launch {
            importMedia(uri)
                .onSuccess { _state.update { it.copy(isEncrypting = false, isEncrypted = true) } }
                .onFailure { e -> _state.update { it.copy(isEncrypting = false, error = e.mediaError) } }
        }
    }

    fun clear() {
        if (_state.value.isEncrypting) return
        _state.value = EncryptUiState()
    }
}
