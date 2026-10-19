package org.anon.mediaplayer.data.video

import android.media.MediaDataSource
import java.io.IOException
import org.anon.mediaplayer.domain.model.MediaException

/** Feeds `MediaMetadataRetriever` from an encrypted video, chunk by chunk. */
class EncryptedMediaDataSource(
    private val reader: ChunkedPlaintextReader,
) : MediaDataSource() {

    /** The retriever swallows read errors, so the first one is kept here. */
    @Volatile
    var failure: MediaException? = null
        private set

    override fun readAt(position: Long, buffer: ByteArray, offset: Int, size: Int): Int =
        try {
            reader.read(position, buffer, offset, size)
        } catch (e: MediaException) {
            if (failure == null) failure = e
            throw IOException(e.message, e)
        }

    override fun getSize(): Long = reader.plaintextSize

    override fun close() = reader.clear()
}
