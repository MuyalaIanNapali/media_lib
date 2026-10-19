package org.anon.mediaplayer.playback

import androidx.media3.common.PlaybackException
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException

/** Recovers the [MediaError] behind a playback failure. */
fun PlaybackException.toMediaError(): MediaError {
    generateSequence<Throwable>(this) { it.cause }
        .filterIsInstance<MediaException>()
        .firstOrNull()
        ?.let { return it.error }

    return when (errorCode / 1000) {
        3, 4 -> MediaError.UNSUPPORTED_MEDIA // parsing (3xxx) or decoding (4xxx)
        else -> MediaError.UNKNOWN
    }
}
