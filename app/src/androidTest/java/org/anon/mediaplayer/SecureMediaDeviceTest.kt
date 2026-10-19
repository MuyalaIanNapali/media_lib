package org.anon.mediaplayer

import android.content.Context
import android.graphics.BitmapFactory
import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import android.os.ParcelFileDescriptor
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.anon.mediaplayer.data.crypto.MediaErrorMapper
import org.anon.mediaplayer.data.crypto.MediaLibNativeCrypto
import org.anon.mediaplayer.data.crypto.NativeErrorCode
import org.anon.mediaplayer.data.local.EncryptedMediaRef
import org.anon.mediaplayer.data.thumbnail.ImageThumbnailGenerator
import org.anon.mediaplayer.data.thumbnail.VideoThumbnailGenerator
import org.anon.mediaplayer.data.video.ChunkedPlaintextReader
import org.anon.mediaplayer.data.crypto.NativeImageReader
import org.anon.mediaplayer.data.video.NativePlaintextReaderFactory
import org.anon.mediaplayer.playback.EncryptedDataSource
import org.anon.mediaplayer.data.video.EncryptedMediaDataSource
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Exercises the real Rust library on a device: encryption, whole-image
 * decryption, chunked video reads, thumbnails, ExoPlayer playback and the
 * typed `MediaLibException` thrown across JNI.
 */
@RunWith(AndroidJUnit4::class)
class SecureMediaDeviceTest {

    private val context: Context = InstrumentationRegistry.getInstrumentation().targetContext
    private val crypto = MediaLibNativeCrypto()
    private val readerFactory = NativePlaintextReaderFactory(crypto)
    private lateinit var dir: File

    @Before
    fun setUp() {
        dir = File(context.cacheDir, "device_test").apply { deleteRecursively(); mkdirs() }
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
    }

    private fun asset(name: String): File {
        val out = File(dir, "plain_$name")
        InstrumentationRegistry.getInstrumentation().context.assets.open(name).use { input ->
            out.outputStream().use { input.copyTo(it) }
        }
        return out
    }

    private fun encrypt(plain: File, mime: String, type: MediaType): EncryptedMediaRef {
        val name = "${plain.name}.enc"
        val fd = ParcelFileDescriptor.open(plain, ParcelFileDescriptor.MODE_READ_ONLY).detachFd()
        val key = crypto.stripAndEncrypt(fd, mime, dir.absolutePath, name)
        assertEquals(32, key.size)
        return EncryptedMediaRef(1, type, File(dir, name), key)
    }

    private fun flipByte(file: File, offset: Long) = RandomAccessFile(file, "rw").use {
        it.seek(offset)
        val b = it.read()
        it.seek(offset)
        it.write(b xor 0xFF)
    }

    private fun expectNativeError(code: NativeErrorCode, block: () -> Unit): MediaLibException {
        try {
            block()
        } catch (e: MediaLibException) {
            assertEquals("unexpected error: ${e.message}", code, NativeErrorCode.fromCode(e.code))
            return e
        }
        fail("expected MediaLibException $code")
        error("unreachable")
    }

    private fun indexOf(haystack: ByteArray, needle: String): Int {
        val n = needle.toByteArray(Charsets.ISO_8859_1)
        return (0..haystack.size - n.size).firstOrNull { i ->
            n.indices.all { haystack[i + it] == n[it] }
        } ?: -1
    }

    private fun ByteArray.contains(needle: String) = indexOf(this, needle) >= 0

    /** Whole plaintext of an encrypted audio/video item (test-sized files only). */
    private fun decryptAll(ref: EncryptedMediaRef): ByteArray {
        val size = readerFactory.create(ref).plaintextSize
        return crypto.readRange(ref.directory, ref.fileName, 0, size, ref.key)
    }

    @Test
    fun imageDecryptsWholeFileInMemory() {
        val plain = asset("sample.jpg")
        val ref = encrypt(plain, "image/jpeg", MediaType.IMAGE)

        val decrypted = crypto.decrypt(ref.directory, ref.fileName, ref.key)
        // ffmpeg's "Lavc" encoder comment is metadata and gets stripped.
        assertTrue(plain.readBytes().contains("Lavc"))
        assertFalse(decrypted.contains("Lavc"))
        assertNotNull(BitmapFactory.decodeByteArray(decrypted, 0, decrypted.size))

        val thumb = ImageThumbnailGenerator(NativeImageReader(crypto)).generate(ref, 512)
        assertTrue(maxOf(thumb.width, thumb.height) <= 512)
    }

    @Test
    fun videoRangeReadsMatchPlaintext() {
        val plain = asset("sample.mp4")
        val bytes = plain.readBytes()
        val ref = encrypt(plain, "video/mp4", MediaType.VIDEO)
        val reader = readerFactory.create(ref)
        val stripped = decryptAll(ref)

        assertEquals(stripped.size.toLong(), reader.plaintextSize)
        assertTrue("sample must span several chunks", stripped.size > 3 * ChunkedPlaintextReader.BLOCK_SIZE)

        // Only moov changed: the media data (mdat to the end) is byte-identical.
        val original = bytes.copyOfRange(indexOf(bytes, "mdat"), bytes.size)
        assertArrayEquals(original, stripped.copyOfRange(indexOf(stripped, "mdat"), stripped.size))

        // A range straddling the chunk 1/2 boundary.
        val start = 2L * ChunkedPlaintextReader.BLOCK_SIZE - 1000
        val buffer = ByteArray(5000)
        assertEquals(5000, reader.read(start, buffer, 0, 5000))
        assertArrayEquals(stripped.copyOfRange(start.toInt(), start.toInt() + 5000), buffer)
        assertEquals(2, reader.blocksDecrypted)
    }

    @Test
    fun videoThumbnailDecryptsOnlyAFewChunks() {
        val plain = asset("sample.mp4")
        val ref = encrypt(plain, "video/mp4", MediaType.VIDEO)

        val reader = readerFactory.create(ref)
        val totalChunks = (reader.plaintextSize + ChunkedPlaintextReader.BLOCK_SIZE - 1) /
            ChunkedPlaintextReader.BLOCK_SIZE
        val frame = MediaMetadataRetriever().use { retriever ->
            retriever.setDataSource(EncryptedMediaDataSource(reader))
            retriever.getScaledFrameAtTime(0, MediaMetadataRetriever.OPTION_CLOSEST_SYNC, 512, 512)
        }
        assertNotNull(frame)
        assertTrue(
            "decrypted ${reader.blocksDecrypted} of $totalChunks chunks",
            reader.blocksDecrypted < totalChunks,
        )

        // And through the production generator.
        val thumb = VideoThumbnailGenerator(readerFactory).generate(ref, 512)
        assertTrue(maxOf(thumb.width, thumb.height) <= 512)
    }

    @OptIn(UnstableApi::class)
    @Test
    fun exoPlayerPlaysAndSeeksEncryptedVideo() {
        val ref = encrypt(asset("sample.mp4"), "video/mp4", MediaType.VIDEO)
        val reader = readerFactory.create(ref)
        val instrumentation = InstrumentationRegistry.getInstrumentation()

        var player: ExoPlayer? = null
        var playbackError: PlaybackException? = null
        val ready = CountDownLatch(1)
        val readyAfterSeek = CountDownLatch(1)

        instrumentation.runOnMainSync {
            player = ExoPlayer.Builder(context).build().apply {
                addListener(object : Player.Listener {
                    override fun onPlaybackStateChanged(state: Int) {
                        if (state != Player.STATE_READY) return
                        if (ready.count > 0) ready.countDown() else readyAfterSeek.countDown()
                    }

                    override fun onPlayerError(error: PlaybackException) {
                        playbackError = error
                        ready.countDown()
                        readyAfterSeek.countDown()
                    }
                })
                setMediaSource(
                    ProgressiveMediaSource.Factory(EncryptedDataSource.Factory(reader))
                        .createMediaSource(MediaItem.fromUri("secure://media/1"))
                )
                prepare()
            }
        }
        try {
            assertTrue("player never became ready", ready.await(15, TimeUnit.SECONDS))
            assertNull(playbackError)

            var duration = 0L
            instrumentation.runOnMainSync {
                val p = checkNotNull(player)
                duration = p.duration
                p.seekTo(6_000)
            }
            assertTrue("duration was $duration", duration in 7_000..9_000)
            assertTrue("seek never completed", readyAfterSeek.await(15, TimeUnit.SECONDS))
            assertNull(playbackError)
        } finally {
            instrumentation.runOnMainSync { player?.release() }
        }
    }

    @Test
    fun tamperedVideoChunkThrowsTypedException() {
        val ref = encrypt(asset("sample.mp4"), "video/mp4", MediaType.VIDEO)
        // 96-byte header + chunk 0 (256 KiB + 32-byte MAC) + 10 bytes into chunk 1.
        flipByte(ref.file, 96L + ChunkedPlaintextReader.BLOCK_SIZE + 32 + 10)

        // Chunk 0 is untouched and still decrypts.
        crypto.readRange(ref.directory, ref.fileName, 0, 100, ref.key)

        val e = expectNativeError(NativeErrorCode.CHUNK_AUTHENTICATION_FAILED) {
            crypto.readRange(ref.directory, ref.fileName, ChunkedPlaintextReader.BLOCK_SIZE.toLong(), 100, ref.key)
        }
        assertEquals("chunk 1 authentication failed", e.message)
        assertEquals(MediaError.AUTHENTICATION_FAILED, MediaErrorMapper.toMediaException(e).error)

        // Through the reader the failure surfaces as a domain MediaException.
        val reader = readerFactory.create(ref)
        try {
            reader.read(ChunkedPlaintextReader.BLOCK_SIZE.toLong(), ByteArray(10), 0, 10)
            fail("expected MediaException")
        } catch (e: MediaException) {
            assertEquals(MediaError.AUTHENTICATION_FAILED, e.error)
        }
    }

    @Test
    fun tamperedHeaderThrowsHeaderAuthenticationFailed() {
        val ref = encrypt(asset("sample.mp4"), "video/mp4", MediaType.VIDEO)
        flipByte(ref.file, 30) // inside the IV field
        expectNativeError(NativeErrorCode.HEADER_AUTHENTICATION_FAILED) {
            crypto.readRange(ref.directory, ref.fileName, 0, 100, ref.key)
        }
    }

    @Test
    fun tamperedOrTruncatedImageThrowsTypedException() {
        val ref = encrypt(asset("sample.jpg"), "image/jpeg", MediaType.IMAGE)
        flipByte(ref.file, 24 + 4 + 10)
        expectNativeError(NativeErrorCode.CHUNK_AUTHENTICATION_FAILED) {
            crypto.decrypt(ref.directory, ref.fileName, ref.key)
        }

        val truncated = encrypt(asset("sample.jpg"), "image/jpeg", MediaType.IMAGE)
        RandomAccessFile(truncated.file, "rw").use { it.setLength(it.length() - 20) }
        expectNativeError(NativeErrorCode.TRUNCATED) {
            crypto.decrypt(truncated.directory, truncated.fileName, truncated.key)
        }
    }

    @Test
    fun wrongKeyAndBadInputsThrowTypedExceptions() {
        val ref = encrypt(asset("sample.jpg"), "image/jpeg", MediaType.IMAGE)
        expectNativeError(NativeErrorCode.CHUNK_AUTHENTICATION_FAILED) {
            crypto.decrypt(ref.directory, ref.fileName, ByteArray(32))
        }
        expectNativeError(NativeErrorCode.INVALID_KEY_LENGTH) {
            crypto.decrypt(ref.directory, ref.fileName, ByteArray(16))
        }
        expectNativeError(NativeErrorCode.IO) {
            crypto.decrypt(ref.directory, "missing.enc", ref.key)
        }

        val plain = asset("sample.jpg")
        val fd = ParcelFileDescriptor.open(plain, ParcelFileDescriptor.MODE_READ_ONLY).detachFd()
        expectNativeError(NativeErrorCode.UNSUPPORTED_MIME_TYPE) {
            crypto.stripAndEncrypt(fd, "text/plain", dir.absolutePath, "x.enc")
        }
        assertTrue("no partial output left behind", dir.listFiles()!!.none { it.name.startsWith("x.enc") })

        val video = encrypt(asset("sample.mp4"), "video/mp4", MediaType.VIDEO)
        expectNativeError(NativeErrorCode.INVALID_RANGE) {
            crypto.readRange(video.directory, video.fileName, -1, 10, video.key)
        }
    }

    // ------------------------------------------------------------------
    // Metadata removal and audio
    // ------------------------------------------------------------------

    private val tagStrings = listOf("secret", "GPSSECRET", "PixelSecret", "+12.3456")

    private fun assertNoTags(name: String, plaintext: ByteArray) {
        tagStrings.forEach { assertFalse("$name still contains \"$it\"", plaintext.contains(it)) }
    }

    @Test
    fun jpegLosesGpsAndDeviceButKeepsOrientation() {
        val plain = asset("tagged.jpg")
        assertTrue(plain.readBytes().contains("GPSSECRET"))
        val ref = encrypt(plain, "image/jpeg", MediaType.IMAGE)

        val decrypted = crypto.decrypt(ref.directory, ref.fileName, ref.key)
        assertNoTags("tagged.jpg", decrypted)
        val exif = ExifInterface(decrypted.inputStream())
        assertEquals(ExifInterface.ORIENTATION_ROTATE_90, exif.getAttributeInt(ExifInterface.TAG_ORIENTATION, 0))
        assertNull(exif.getAttribute(ExifInterface.TAG_MAKE))
        assertNull(exif.getAttribute(ExifInterface.TAG_GPS_LATITUDE_REF))
        assertNotNull(BitmapFactory.decodeByteArray(decrypted, 0, decrypted.size))
    }

    @Test
    fun videoLosesLocationAndTitle() {
        val ref = encrypt(asset("tagged.mp4"), "video/mp4", MediaType.VIDEO)
        assertNoTags("tagged.mp4", decryptAll(ref))
        // Still a valid video: a preview frame can be extracted.
        assertNotNull(VideoThumbnailGenerator(readerFactory).generate(ref, 256))
    }

    @Test
    fun audioFormatsLoseTagsAndStillPlay() {
        listOf(
            "tagged.m4a" to "audio/mp4",
            "tagged.mp3" to "audio/mpeg",
            "tagged.flac" to "audio/flac",
        ).forEach { (name, mime) ->
            val ref = encrypt(asset(name), mime, MediaType.AUDIO)
            assertNoTags(name, decryptAll(ref))
            assertPlays(name, readerFactory.create(ref))
        }
    }

    @Test
    fun formatsWithoutAStripperAreRefused() {
        val plain = asset("sample.ogg")
        val fd = ParcelFileDescriptor.open(plain, ParcelFileDescriptor.MODE_READ_ONLY).detachFd()
        val e = expectNativeError(NativeErrorCode.UNSUPPORTED_CONTAINER) {
            crypto.stripAndEncrypt(fd, "audio/ogg", dir.absolutePath, "ogg.enc")
        }
        assertEquals(MediaError.METADATA_UNSUPPORTED, MediaErrorMapper.toMediaException(e).error)
        assertTrue("no output left behind", dir.listFiles()!!.none { it.name.startsWith("ogg.enc") })
    }

    @OptIn(UnstableApi::class)
    private fun assertPlays(name: String, reader: ChunkedPlaintextReader) {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        var player: ExoPlayer? = null
        var playbackError: PlaybackException? = null
        val ready = CountDownLatch(1)
        instrumentation.runOnMainSync {
            player = ExoPlayer.Builder(context).build().apply {
                addListener(object : Player.Listener {
                    override fun onPlaybackStateChanged(state: Int) {
                        if (state == Player.STATE_READY) ready.countDown()
                    }

                    override fun onPlayerError(error: PlaybackException) {
                        playbackError = error
                        ready.countDown()
                    }
                })
                setMediaSource(
                    ProgressiveMediaSource.Factory(EncryptedDataSource.Factory(reader))
                        .createMediaSource(MediaItem.fromUri("secure://media/$name"))
                )
                prepare()
            }
        }
        try {
            assertTrue("$name never became ready", ready.await(15, TimeUnit.SECONDS))
            assertNull("$name failed: $playbackError", playbackError)
        } finally {
            instrumentation.runOnMainSync { player?.release() }
        }
    }
}
