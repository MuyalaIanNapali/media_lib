package org.anon.mediaplayer.playback

import android.net.Uri
import androidx.media3.common.C
import androidx.media3.common.PlaybackException
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.BaseDataSource
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSourceException
import androidx.media3.datasource.DataSpec
import java.io.IOException
import kotlin.math.min
import org.anon.mediaplayer.data.video.ChunkedPlaintextReader
import org.anon.mediaplayer.domain.model.MediaException

/**
 * Media3 [DataSource] that streams plaintext from an encrypted video. Seeking
 * works because the AES-CTR chunks are independently addressable.
 */
@UnstableApi
class EncryptedDataSource(
    private val reader: ChunkedPlaintextReader,
) : BaseDataSource(/* isNetwork = */ false) {

    private var uri: Uri? = null
    private var position = 0L
    private var bytesRemaining = 0L
    private var opened = false

    override fun open(dataSpec: DataSpec): Long {
        uri = dataSpec.uri
        transferInitializing(dataSpec)

        if (dataSpec.position > reader.plaintextSize) {
            throw DataSourceException(PlaybackException.ERROR_CODE_IO_READ_POSITION_OUT_OF_RANGE)
        }
        position = dataSpec.position
        bytesRemaining = if (dataSpec.length != C.LENGTH_UNSET.toLong()) {
            dataSpec.length
        } else {
            reader.plaintextSize - position
        }

        opened = true
        transferStarted(dataSpec)
        return bytesRemaining
    }

    override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
        if (length == 0) return 0
        if (bytesRemaining == 0L) return C.RESULT_END_OF_INPUT

        val toRead = min(length.toLong(), bytesRemaining).toInt()
        val read = try {
            reader.read(position, buffer, offset, toRead)
        } catch (e: MediaException) {
            // Keeps the MediaException in the cause chain for toMediaError().
            throw IOException(e.message, e)
        }
        if (read == -1) return C.RESULT_END_OF_INPUT

        position += read
        bytesRemaining -= read
        bytesTransferred(read)
        return read
    }

    override fun getUri(): Uri? = uri

    override fun close() {
        // Don't keep plaintext around between loads; the next read re-decrypts.
        reader.clear()
        uri = null
        if (opened) {
            opened = false
            transferEnded()
        }
    }

    /** Shares one [reader] (and its block cache) across every source it creates. */
    class Factory(private val reader: ChunkedPlaintextReader) : DataSource.Factory {
        override fun createDataSource(): DataSource = EncryptedDataSource(reader)
    }
}
