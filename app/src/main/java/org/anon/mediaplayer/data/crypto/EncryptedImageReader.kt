package org.anon.mediaplayer.data.crypto

import javax.inject.Inject
import org.anon.mediaplayer.data.local.EncryptedMediaRef

/** Decrypts a whole image into memory; nothing is written to disk. */
interface EncryptedImageReader {
    /** Blocking. */
    fun read(ref: EncryptedMediaRef): ByteArray
}

class NativeImageReader @Inject constructor(
    private val crypto: NativeMediaCrypto,
) : EncryptedImageReader {

    override fun read(ref: EncryptedMediaRef): ByteArray =
        mapMediaErrors { crypto.decrypt(ref.directory, ref.fileName, ref.key) }
}
