package org.anon.mediaplayer.data.crypto

import javax.inject.Inject
import org.anon.mediaplayer.MediaLib

/** Abstraction over the JNI [MediaLib] so the data layer can be tested with fakes. */
interface NativeMediaCrypto {
    fun stripAndEncrypt(fd: Int, mimeType: String, outputDir: String, fileName: String): ByteArray
    fun decrypt(dir: String, fileName: String, key: ByteArray): ByteArray
    fun readRange(dir: String, fileName: String, position: Long, length: Long, key: ByteArray): ByteArray
}

class MediaLibNativeCrypto @Inject constructor() : NativeMediaCrypto {

    override fun stripAndEncrypt(fd: Int, mimeType: String, outputDir: String, fileName: String) =
        MediaLib.nativeStripAndEncrypt(fd, mimeType, outputDir, fileName)

    override fun decrypt(dir: String, fileName: String, key: ByteArray) =
        MediaLib.nativeDecrypt(dir, fileName, key)

    override fun readRange(dir: String, fileName: String, position: Long, length: Long, key: ByteArray) =
        MediaLib.nativeReadMedia(dir, fileName, position, length, key)
}
