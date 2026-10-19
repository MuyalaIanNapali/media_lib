package org.anon.mediaplayer.data.crypto

import java.io.FileNotFoundException
import java.io.IOException
import kotlinx.coroutines.CancellationException
import org.anon.mediaplayer.MediaLibException
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException

/** The single place that translates native and I/O failures into [MediaError]. */
object MediaErrorMapper {

    fun map(code: NativeErrorCode): MediaError = when (code) {
        NativeErrorCode.HEADER_AUTHENTICATION_FAILED,
        NativeErrorCode.CHUNK_AUTHENTICATION_FAILED,
        NativeErrorCode.INVALID_KEY_LENGTH -> MediaError.AUTHENTICATION_FAILED

        NativeErrorCode.INVALID_HEADER,
        NativeErrorCode.UNSUPPORTED_FORMAT,
        NativeErrorCode.TRUNCATED,
        NativeErrorCode.TRAILING_DATA,
        NativeErrorCode.INVALID_CHUNK_LENGTH,
        NativeErrorCode.FILE_SIZE_MISMATCH,
        NativeErrorCode.MALFORMED_CONTAINER -> MediaError.CORRUPTED

        NativeErrorCode.UNSUPPORTED_MIME_TYPE,
        NativeErrorCode.CONTAINER_TOO_LARGE -> MediaError.UNSUPPORTED_MEDIA

        NativeErrorCode.UNSUPPORTED_CONTAINER -> MediaError.METADATA_UNSUPPORTED

        NativeErrorCode.IO -> MediaError.STORAGE

        NativeErrorCode.INVALID_RANGE,
        NativeErrorCode.KEY_DERIVATION,
        NativeErrorCode.ENCRYPTION_FAILED,
        NativeErrorCode.JNI,
        NativeErrorCode.UNKNOWN -> MediaError.UNKNOWN
    }

    fun toMediaException(throwable: Throwable): MediaException = when (throwable) {
        is MediaException -> throwable
        is MediaLibException -> MediaException(
            map(NativeErrorCode.fromCode(throwable.code)), throwable.message, throwable
        )
        is FileNotFoundException -> MediaException(MediaError.NOT_FOUND, throwable.message, throwable)
        is IOException -> MediaException(MediaError.STORAGE, throwable.message, throwable)
        else -> MediaException(MediaError.UNKNOWN, throwable.message, throwable)
    }
}

/** Runs [block], rethrowing any failure as a [MediaException]. */
inline fun <T> mapMediaErrors(block: () -> T): T =
    try {
        block()
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        throw MediaErrorMapper.toMediaException(e)
    }
