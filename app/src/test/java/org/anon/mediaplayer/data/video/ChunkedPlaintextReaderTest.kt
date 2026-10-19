package org.anon.mediaplayer.data.video

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChunkedPlaintextReaderTest {

    private val blockSize = 16
    private val plaintext = ByteArray(100) { it.toByte() }

    private val requests = mutableListOf<Pair<Long, Int>>()
    private val handedOut = mutableListOf<ByteArray>()
    private val source = PlaintextRangeSource { position, length ->
        requests += position to length
        plaintext.copyOfRange(position.toInt(), position.toInt() + length).also { handedOut += it }
    }

    private fun reader(cachedBlocks: Int = 2) =
        ChunkedPlaintextReader(source, plaintext.size.toLong(), blockSize, cachedBlocks)

    private fun ChunkedPlaintextReader.readBytes(position: Long, length: Int): ByteArray {
        val buffer = ByteArray(length)
        val read = read(position, buffer, 0, length)
        return buffer.copyOf(read)
    }

    @Test
    fun `reads within one block`() {
        assertArrayEquals(plaintext.copyOfRange(3, 9), reader().readBytes(3, 6))
        assertEquals(listOf(0L to blockSize), requests)
    }

    @Test
    fun `reads across block boundaries`() {
        assertArrayEquals(plaintext.copyOfRange(10, 40), reader().readBytes(10, 30))
        assertEquals(listOf(0L to 16, 16L to 16, 32L to 16), requests)
    }

    @Test
    fun `requests are block aligned and the last block is short`() {
        val r = reader()
        assertArrayEquals(plaintext.copyOfRange(97, 100), r.readBytes(97, 50))
        assertEquals(listOf(96L to 4), requests)
    }

    @Test
    fun `cached block is not decrypted twice`() {
        val r = reader()
        r.readBytes(0, 4)
        r.readBytes(4, 4)
        r.readBytes(12, 2)
        assertEquals(1, r.blocksDecrypted)
    }

    @Test
    fun `least recently used block is evicted`() {
        val r = reader(cachedBlocks = 2)
        r.readBytes(0, 1)   // block 0
        r.readBytes(16, 1)  // block 1
        r.readBytes(32, 1)  // block 2 evicts block 0
        r.readBytes(0, 1)   // block 0 again
        assertEquals(4, r.blocksDecrypted)
    }

    @Test
    fun `returns -1 at end of input`() {
        assertEquals(-1, reader().read(100, ByteArray(4), 0, 4))
        assertEquals(-1, reader().read(150, ByteArray(4), 0, 4))
    }

    @Test
    fun `zero length read returns 0 without decrypting`() {
        val r = reader()
        assertEquals(0, r.read(5, ByteArray(0), 0, 0))
        assertEquals(0, r.blocksDecrypted)
    }

    @Test
    fun `honours buffer offset`() {
        val buffer = ByteArray(10)
        val read = reader().read(20, buffer, 4, 6)
        assertEquals(6, read)
        assertArrayEquals(plaintext.copyOfRange(20, 26), buffer.copyOfRange(4, 10))
    }

    @Test
    fun `default cache holds a single block`() {
        val r = ChunkedPlaintextReader(source, plaintext.size.toLong(), blockSize)
        r.readBytes(0, 50) // walks blocks 0..3, ending on a full block
        assertEquals(blockSize, r.cachedBytes)
    }

    @Test
    fun `sequential read decrypts each block once with one cached block`() {
        val r = ChunkedPlaintextReader(source, plaintext.size.toLong(), blockSize)
        for (position in 0L until 100L step 5) r.readBytes(position, 5)
        assertEquals(7, r.blocksDecrypted) // ceil(100 / 16)
    }

    @Test
    fun `evicted blocks are zeroed`() {
        val r = ChunkedPlaintextReader(source, plaintext.size.toLong(), blockSize)
        r.readBytes(20, 1) // block 1
        r.readBytes(40, 1) // block 2 evicts block 1
        assertTrue(handedOut[0].all { it == 0.toByte() })
        assertTrue(handedOut[1].any { it != 0.toByte() })
    }

    @Test
    fun `clear zeroes and releases every block`() {
        val r = reader(cachedBlocks = 2)
        r.readBytes(10, 20) // blocks 0 and 1
        r.clear()
        assertEquals(0, r.cachedBytes)
        handedOut.forEach { block -> assertTrue(block.all { it == 0.toByte() }) }
        // Still readable afterwards.
        assertArrayEquals(plaintext.copyOfRange(10, 30), r.readBytes(10, 20))
    }
}
