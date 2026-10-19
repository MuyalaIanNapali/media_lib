package org.anon.mediaplayer.data

import android.graphics.Bitmap
import java.io.File
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.anon.mediaplayer.data.crypto.EncryptedFile
import org.anon.mediaplayer.data.crypto.MediaEncryptor
import org.anon.mediaplayer.data.local.EncryptedFileStore
import org.anon.mediaplayer.data.local.EncryptedMediaRef
import org.anon.mediaplayer.data.local.MediaDao
import org.anon.mediaplayer.data.local.MediaEntity
import org.anon.mediaplayer.data.local.MediaLocator
import org.anon.mediaplayer.data.repository.SecureMediaRepositoryImpl
import org.anon.mediaplayer.data.thumbnail.ThumbnailCache
import org.anon.mediaplayer.data.thumbnail.ThumbnailEncoder
import org.anon.mediaplayer.data.thumbnail.ThumbnailGenerator
import org.anon.mediaplayer.data.thumbnail.ThumbnailProviderImpl
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.fail
import org.junit.Test

class DataLayerTest {

    private val dispatcher = StandardTestDispatcher()

    // ---- fakes ---------------------------------------------------------

    private class FakeStore : EncryptedFileStore {
        val deleted = mutableListOf<String>()
        override fun ensureDirectory() = File("/tmp")
        override fun newFileName() = "new.enc"
        override fun file(fileName: String) = File("/tmp", fileName)
        override fun delete(fileName: String) {
            deleted += fileName
        }
    }

    private class FakeDao(private val insert: (MediaEntity) -> Long) : MediaDao {
        override suspend fun insert(media: MediaEntity) = insert.invoke(media)
        override suspend fun getById(id: Long): MediaEntity? = null
        override fun observeAll(): Flow<List<MediaEntity>> = flowOf(emptyList())
        override suspend fun deleteById(id: Long) = Unit
    }

    private val encryptor = object : MediaEncryptor {
        override fun encrypt(uri: String) = EncryptedFile("song.enc", "audio/mpeg", ByteArray(32))
    }

    private class FakeLocator(private val type: MediaType) : MediaLocator {
        var calls = 0
        override suspend fun locate(mediaId: Long, expectedTypes: Set<MediaType>): EncryptedMediaRef {
            calls++
            return EncryptedMediaRef(mediaId, type, File("/tmp/x.enc"), ByteArray(32))
        }
    }

    private class MapCache : ThumbnailCache {
        val map = mutableMapOf<Long, ByteArray>()
        override fun get(mediaId: Long) = map[mediaId]
        override fun put(mediaId: Long, thumbnail: ByteArray) {
            map[mediaId] = thumbnail
        }
    }

    private val unusedEncoder = object : ThumbnailEncoder {
        override fun encode(bitmap: Bitmap): ByteArray = throw AssertionError("encoder not expected")
    }

    // ---- repository ----------------------------------------------------

    @Test
    fun `import records the encrypted file`() = runTest(dispatcher) {
        var saved: MediaEntity? = null
        val repository = SecureMediaRepositoryImpl(FakeDao { saved = it; 5 }, encryptor, FakeStore(), dispatcher)

        val media = repository.import("content://song")

        assertEquals(5L, media.id)
        assertEquals(MediaType.AUDIO, media.type)
        val entity = checkNotNull(saved)
        assertEquals("song.enc", entity.fileName)
        assertEquals("audio/mpeg", entity.mimeType)
    }

    @Test
    fun `failed insert deletes the orphaned file`() = runTest(dispatcher) {
        val store = FakeStore()
        val repository = SecureMediaRepositoryImpl(FakeDao { error("disk full") }, encryptor, store, dispatcher)

        try {
            repository.import("content://song")
            fail("expected MediaException")
        } catch (e: MediaException) {
            assertEquals(MediaError.STORAGE, e.error)
        }
        assertEquals(listOf("song.enc"), store.deleted)
    }

    // ---- thumbnails ----------------------------------------------------

    @Test
    fun `cached thumbnail skips lookup and generation`() = runTest(dispatcher) {
        val locator = FakeLocator(MediaType.IMAGE)
        val cache = MapCache().apply { map[1L] = byteArrayOf(9) }
        val provider = ThumbnailProviderImpl(locator, emptySet(), cache, unusedEncoder, dispatcher)

        assertArrayEquals(byteArrayOf(9), provider.thumbnail(1))
        assertEquals(0, locator.calls)
    }

    @Test
    fun `type without a generator is unsupported`() = runTest(dispatcher) {
        val imageOnly = object : ThumbnailGenerator {
            override val type = MediaType.IMAGE
            override fun generate(ref: EncryptedMediaRef, maxSize: Int): Bitmap = throw AssertionError("not expected")
        }
        val provider = ThumbnailProviderImpl(FakeLocator(MediaType.AUDIO), setOf(imageOnly), MapCache(), unusedEncoder, dispatcher)

        try {
            provider.thumbnail(1)
            fail("expected MediaException")
        } catch (e: MediaException) {
            assertEquals(MediaError.UNSUPPORTED_MEDIA, e.error)
        }
    }
}
