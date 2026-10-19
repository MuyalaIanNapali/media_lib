package org.anon.mediaplayer.domain.usecase

import kotlinx.coroutines.CancellationException
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException

/**
 * Runs [block], turning any failure into a [Result] whose exception is always a
 * [MediaException]. Cancellation is rethrown so structured concurrency works.
 */
internal suspend inline fun <T> mediaResultOf(crossinline block: suspend () -> T): Result<T> =
    try {
        Result.success(block())
    } catch (e: CancellationException) {
        throw e
    } catch (e: MediaException) {
        Result.failure(e)
    } catch (e: Exception) {
        Result.failure(MediaException(MediaError.UNKNOWN, e.message, e))
    }

/** The [MediaError] carried by a failed [Result] from a use case. */
val Throwable.mediaError: MediaError
    get() = (this as? MediaException)?.error ?: MediaError.UNKNOWN
