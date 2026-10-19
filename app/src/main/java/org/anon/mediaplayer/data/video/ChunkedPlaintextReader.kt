package org.anon.mediaplayer.data.video

import kotlin.math.min

/** Decrypts `length` plaintext bytes starting at `position`. */
fun interface PlaintextRangeSource {
    fun read(position: Long, length: Int): ByteArray
}

/**
 * Random-access reader over an encrypted video that only ever decrypts the
 * blocks a caller touches.
 *
 * Players and `MediaMetadataRetriever` issue many small reads, while each
 * native read authenticates and decrypts a whole chunk anyway. So reads are
 * aligned to [blockSize] (the Rust `CHUNK_SIZE`) and only the most recent
 * [maxCachedBlocks] are kept: one for sequential playback, a couple for
 * thumbnail extraction, which hops between the index and the first frame.
 * Blocks are zeroed when evicted or cleared, and never written to disk.
 *
 * Thread-safe: one reader can back several data sources.
 */
class ChunkedPlaintextReader(
    private val source: PlaintextRangeSource,
    val plaintextSize: Long,
    private val blockSize: Int = BLOCK_SIZE,
    private val maxCachedBlocks: Int = 1,
) {
    init {
        require(maxCachedBlocks >= 1) { "at least one block must be cacheable" }
    }

    private val cache = object : LinkedHashMap<Long, ByteArray>(maxCachedBlocks + 1, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<Long, ByteArray>?): Boolean {
            val evict = size > maxCachedBlocks
            if (evict) eldest?.value?.fill(0)
            return evict
        }
    }

    /** Plaintext bytes currently held. */
    val cachedBytes: Int
        @Synchronized get() = cache.values.sumOf { it.size }

    /** Number of blocks decrypted so far: lets callers verify reads stay partial. */
    var blocksDecrypted: Int = 0
        private set

    /**
     * Copies up to [length] bytes at [position] into [buffer]. Returns the
     * number of bytes copied, or -1 if [position] is at or past the end.
     */
    @Synchronized
    fun read(position: Long, buffer: ByteArray, offset: Int, length: Int): Int {
        require(position >= 0) { "position must be non-negative" }
        if (length == 0) return 0
        if (position >= plaintextSize) return -1

        var copied = 0
        var current = position
        while (copied < length && current < plaintextSize) {
            val index = current / blockSize
            val block = block(index)
            val inBlock = (current - index * blockSize).toInt()
            if (inBlock >= block.size) break

            val count = min(length - copied, block.size - inBlock)
            System.arraycopy(block, inBlock, buffer, offset + copied, count)
            copied += count
            current += count
        }
        return if (copied == 0) -1 else copied
    }

    @Synchronized
    fun clear() {
        cache.values.forEach { it.fill(0) }
        cache.clear()
    }

    private fun block(index: Long): ByteArray = cache.getOrPut(index) {
        val start = index * blockSize
        val length = min(blockSize.toLong(), plaintextSize - start).toInt()
        blocksDecrypted++
        source.read(start, length)
    }

    companion object {
        /** Must match `CHUNK_SIZE` in media_lib/src/crypto/header.rs. */
        const val BLOCK_SIZE = 256 * 1024
    }
}
