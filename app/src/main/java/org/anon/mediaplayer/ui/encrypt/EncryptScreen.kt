package org.anon.mediaplayer.ui.encrypt

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.hilt.lifecycle.viewmodel.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil.compose.AsyncImage
import org.anon.mediaplayer.R
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.ui.common.messageRes

@Composable
fun EncryptRoute(
    onOpenSecureFolder: () -> Unit,
    viewModel: EncryptViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()

    val pickMedia = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickVisualMedia()
    ) { uri -> viewModel.onMediaPicked(uri?.toString()) }

    // The photo picker only covers images and video; audio comes from the
    // document picker.
    val pickAudio = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri -> viewModel.onMediaPicked(uri?.toString()) }

    EncryptScreen(
        state = state,
        onPick = {
            pickMedia.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageAndVideo)
            )
        },
        onPickAudio = { pickAudio.launch(arrayOf("audio/*")) },
        onEncrypt = viewModel::encrypt,
        onClear = viewModel::clear,
        onOpenSecureFolder = onOpenSecureFolder,
    )
}

@Composable
fun EncryptScreen(
    state: EncryptUiState,
    onPick: () -> Unit,
    onPickAudio: () -> Unit,
    onEncrypt: () -> Unit,
    onClear: () -> Unit,
    onOpenSecureFolder: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = stringResource(R.string.encrypt_title),
            style = MaterialTheme.typography.headlineSmall
        )

        Spacer(modifier = Modifier.height(16.dp))

        Button(enabled = !state.isEncrypting, onClick = onPick) {
            Text(stringResource(R.string.encrypt_choose))
        }

        Spacer(modifier = Modifier.height(8.dp))

        OutlinedButton(enabled = !state.isEncrypting, onClick = onPickAudio) {
            Text(stringResource(R.string.encrypt_choose_audio))
        }

        Spacer(modifier = Modifier.height(16.dp))

        val uri = state.selectedUri
        if (uri == null) {
            Text(
                text = stringResource(R.string.encrypt_none),
                style = MaterialTheme.typography.bodyMedium
            )
            return@Column
        }

        when (state.mediaType) {
            MediaType.VIDEO -> SelectedMediaLabel("▶", stringResource(R.string.encrypt_video_selected))
            MediaType.AUDIO -> SelectedMediaLabel("♪", stringResource(R.string.encrypt_audio_selected))
            else -> AsyncImage(
                model = uri,
                contentDescription = stringResource(R.string.encrypt_selected_image),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(250.dp),
                contentScale = ContentScale.Fit
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        Button(enabled = !state.isEncrypting && !state.isEncrypted, onClick = onEncrypt) {
            Text(stringResource(R.string.encrypt_action))
        }

        if (state.isEncrypting) {
            Spacer(modifier = Modifier.height(12.dp))
            CircularProgressIndicator()
            Text(stringResource(R.string.encrypt_in_progress))
        }

        if (state.isEncrypted) {
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = stringResource(R.string.encrypt_success),
                style = MaterialTheme.typography.titleMedium
            )
            TextButton(onClick = onOpenSecureFolder) {
                Text(stringResource(R.string.encrypt_view_folder))
            }
        }

        state.error?.let { error ->
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.encrypt_failed, stringResource(error.messageRes())),
                color = MaterialTheme.colorScheme.error
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        OutlinedButton(enabled = !state.isEncrypting, onClick = onClear) {
            Text(stringResource(R.string.encrypt_clear))
        }
    }
}

@Composable
private fun SelectedMediaLabel(symbol: String, label: String) {
    Text(text = symbol, fontSize = 64.sp, modifier = Modifier.size(100.dp))
    Text(text = label, style = MaterialTheme.typography.bodyLarge)
}
