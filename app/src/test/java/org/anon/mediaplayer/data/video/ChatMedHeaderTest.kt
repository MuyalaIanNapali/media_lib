package org.anon.mediaplayer.data.video

import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ChatMedHeaderTest {

    private fun header(size: Long, magic: String = "CHATMED2"): ByteArray =
        ByteBuffer.allocate(96).order(ByteOrder.LITTLE_ENDIAN).apply {
            put(magic.toByteArray(Charsets.US_ASCII))
            putInt(2)
            putInt(256 * 1024)
            putLong(size)
        }.array()

    @Test
    fun `parses little-endian plaintext size`() {
        assertEquals(123_456_789_012L, ChatMedHeader.plaintextSize(header(123_456_789_012L)))
    }

    @Test
    fun `rejects wrong magic`() {
        val e = assertThrows(MediaException::class.java) {
            ChatMedHeader.plaintextSize(header(10, magic = "NOTMEDIA"))
        }
        assertEquals(MediaError.CORRUPTED, e.error)
    }

    @Test
    fun `rejects short header`() {
        val e = assertThrows(MediaException::class.java) {
            ChatMedHeader.plaintextSize(ByteArray(10))
        }
        assertEquals(MediaError.CORRUPTED, e.error)
    }
}
