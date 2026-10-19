package org.anon.mediaplayer.data.video

import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException

/**
 * Reads the plaintext size from a CHATMED2 header (media_lib/src/crypto/header.rs).
 *
 * This read is not authenticated, but every `nativeReadMedia` call verifies the
 * header MAC and the expected file size, so a tampered size fails at the
 * first read.
 */
object ChatMedHeader {
    private val MAGIC = "CHATMED2".toByteArray(Charsets.US_ASCII)
    private const val SIZE_OFFSET = 16
    private const val PREFIX_BYTES = 24

    fun plaintextSize(file: File): Long {
        val prefix = ByteArray(PREFIX_BYTES)
        try {
            RandomAccessFile(file, "r").use { it.readFully(prefix) }
        } catch (e: IOException) {
            throw MediaException(MediaError.CORRUPTED, "Encrypted video header unreadable", e)
        }
        return plaintextSize(prefix)
    }

    fun plaintextSize(prefix: ByteArray): Long {
        if (prefix.size < PREFIX_BYTES || !prefix.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)) {
            throw MediaException(MediaError.CORRUPTED, "Not a CHATMED2 file")
        }
        val size = ByteBuffer.wrap(prefix, SIZE_OFFSET, Long.SIZE_BYTES)
            .order(ByteOrder.LITTLE_ENDIAN)
            .long
        if (size < 0) throw MediaException(MediaError.CORRUPTED, "Negative plaintext size")
        return size
    }
}
