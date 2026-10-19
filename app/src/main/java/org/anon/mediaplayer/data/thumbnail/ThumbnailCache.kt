package org.anon.mediaplayer.data.thumbnail

import android.util.LruCache
import javax.inject.Inject
import javax.inject.Singleton

/** Encoded thumbnails by media id. Implementations must stay in memory. */
interface ThumbnailCache {
    operator fun get(mediaId: Long): ByteArray?

    fun put(mediaId: Long, thumbnail: ByteArray)
}

/** Bounded in-memory cache: decrypted previews never touch disk. */
@Singleton
class LruThumbnailCache @Inject constructor() : ThumbnailCache {

    private val cache = object : LruCache<Long, ByteArray>(MAX_BYTES) {
        override fun sizeOf(key: Long, value: ByteArray) = value.size
    }

    override fun get(mediaId: Long): ByteArray? = cache.get(mediaId)

    override fun put(mediaId: Long, thumbnail: ByteArray) {
        cache.put(mediaId, thumbnail)
    }

    private companion object {
        const val MAX_BYTES = 8 * 1024 * 1024
    }
}
