package org.anon.mediaplayer.data.thumbnail

import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import javax.inject.Inject
import org.anon.mediaplayer.data.local.EncryptedMediaRef
import org.anon.mediaplayer.data.video.PlaintextReaderFactory
import org.anon.mediaplayer.data.video.EncryptedMediaDataSource
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType

/**
 * Extracts a frame through [EncryptedMediaDataSource]: the retriever only pulls
 * the chunks holding the container index and the first keyframe, so the video
 * is never fully decrypted.
 */
class VideoThumbnailGenerator @Inject constructor(
    private val readerFactory: PlaintextReaderFactory,
) : ThumbnailGenerator {

    override val type = MediaType.VIDEO

    override fun generate(ref: EncryptedMediaRef, maxSize: Int): Bitmap {
        // Two blocks: the retriever alternates between the index and frame data.
        val source = EncryptedMediaDataSource(readerFactory.create(ref, cachedBlocks = 2))
        return MediaMetadataRetriever().use { retriever ->
            try {
                retriever.setDataSource(source)
                retriever.getScaledFrameAtTime(
                    PREVIEW_TIME_US, MediaMetadataRetriever.OPTION_CLOSEST_SYNC, maxSize, maxSize,
                ) ?: retriever.getScaledFrameAtTime(
                    0, MediaMetadataRetriever.OPTION_CLOSEST_SYNC, maxSize, maxSize,
                )
            } catch (e: RuntimeException) {
                null
            } ?: throw source.failure
                ?: MediaException(MediaError.UNSUPPORTED_MEDIA, "No frame could be extracted")
        }
    }

    private companion object {
        const val PREVIEW_TIME_US = 1_000_000L
    }
}
