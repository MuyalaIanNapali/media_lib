package org.anon.mediaplayer.data.video

import android.util.Log
import javax.inject.Inject
import org.anon.mediaplayer.data.crypto.NativeMediaCrypto
import org.anon.mediaplayer.data.crypto.mapMediaErrors
import org.anon.mediaplayer.data.local.EncryptedMediaRef

/** Opens random-access plaintext readers over encrypted audio/video. */
interface PlaintextReaderFactory {
    /**
     * Blocking (reads the file header); call off the main thread. Each cached
     * block holds [ChunkedPlaintextReader.BLOCK_SIZE] bytes of plaintext.
     */
    fun create(ref: EncryptedMediaRef, cachedBlocks: Int = 1): ChunkedPlaintextReader
}

class NativePlaintextReaderFactory @Inject constructor(
    private val crypto: NativeMediaCrypto,
) : PlaintextReaderFactory {

    override fun create(ref: EncryptedMediaRef, cachedBlocks: Int): ChunkedPlaintextReader {
        val source = PlaintextRangeSource { position, length ->
            Log.d(TAG, "media ${ref.id}: decrypting $length bytes at $position")
            mapMediaErrors {
                crypto.readRange(ref.directory, ref.fileName, position, length.toLong(), ref.key)
            }
        }
        return ChunkedPlaintextReader(
            source,
            ChatMedHeader.plaintextSize(ref.file),
            maxCachedBlocks = cachedBlocks,
        )
    }

    private companion object {
        const val TAG = "SecureVideo"
    }
}
