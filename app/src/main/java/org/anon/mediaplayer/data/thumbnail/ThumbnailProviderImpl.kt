package org.anon.mediaplayer.data.thumbnail

import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.withContext
import org.anon.mediaplayer.data.crypto.mapMediaErrors
import org.anon.mediaplayer.data.local.MediaLocator
import org.anon.mediaplayer.di.IoDispatcher
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.repository.ThumbnailProvider

/**
 * Coordinates thumbnail creation: picks the [ThumbnailGenerator] for the
 * media's type, encodes its frame and caches the result.
 */
@Singleton
class ThumbnailProviderImpl @Inject constructor(
    private val locator: MediaLocator,
    generators: Set<@JvmSuppressWildcards ThumbnailGenerator>,
    private val cache: ThumbnailCache,
    private val encoder: ThumbnailEncoder,
    @IoDispatcher private val io: CoroutineDispatcher,
) : ThumbnailProvider {

    private val generatorsByType = generators.associateBy { it.type }

    /** Each generation may hold a fully decrypted image; cap how many run at once. */
    private val permits = Semaphore(MAX_CONCURRENT)

    override suspend fun thumbnail(mediaId: Long): ByteArray = permits.withPermit {
        cache[mediaId]?.let { return it }

        val ref = locator.locate(mediaId)
        val generator = generatorsByType[ref.type]
            ?: throw MediaException(MediaError.UNSUPPORTED_MEDIA, "No thumbnail for ${ref.type}")

        val thumbnail = withContext(io) {
            mapMediaErrors { encoder.encode(generator.generate(ref, MAX_SIZE_PX)) }
        }
        cache.put(mediaId, thumbnail)
        thumbnail
    }

    private companion object {
        const val MAX_SIZE_PX = 512
        const val MAX_CONCURRENT = 2
    }
}
