package org.anon.mediaplayer.ui.common

import androidx.annotation.StringRes
import org.anon.mediaplayer.R
import org.anon.mediaplayer.domain.model.MediaError

@StringRes
fun MediaError.messageRes(): Int = when (this) {
    MediaError.AUTHENTICATION_FAILED -> R.string.error_authentication_failed
    MediaError.CORRUPTED -> R.string.error_corrupted
    MediaError.UNSUPPORTED_MEDIA -> R.string.error_unsupported_media
    MediaError.METADATA_UNSUPPORTED -> R.string.error_metadata_unsupported
    MediaError.NOT_FOUND -> R.string.error_not_found
    MediaError.STORAGE -> R.string.error_storage
    MediaError.UNKNOWN -> R.string.error_unknown
}
