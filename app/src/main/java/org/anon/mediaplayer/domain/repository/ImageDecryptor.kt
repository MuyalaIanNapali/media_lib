package org.anon.mediaplayer.domain.repository

interface ImageDecryptor {
    /** Decrypts the whole image into memory; nothing is written to disk. */
    suspend fun decrypt(mediaId: Long): ByteArray
}
