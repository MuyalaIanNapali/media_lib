package org.anon.mediaplayer.data.crypto

import java.io.FileNotFoundException
import java.io.IOException
import org.anon.mediaplayer.MediaLibException
import org.anon.mediaplayer.domain.model.MediaError
import org.junit.Assert.assertEquals
import org.junit.Test

class MediaErrorMapperTest {

    @Test
    fun `native codes match the Rust MediaLibError codes`() {
        // media_lib/src/error.rs `codes_are_stable` asserts the same 1..=18 sequence.
        assertEquals(
            (1..18).toList(),
            NativeErrorCode.entries.filter { it != NativeErrorCode.UNKNOWN }.map { it.code },
        )
    }

    @Test
    fun `unknown native code maps to UNKNOWN`() {
        assertEquals(NativeErrorCode.UNKNOWN, NativeErrorCode.fromCode(999))
        assertEquals(MediaError.UNKNOWN, MediaErrorMapper.toMediaException(MediaLibException(999, "x")).error)
    }

    @Test
    fun `authentication failures map to AUTHENTICATION_FAILED`() {
        listOf(6, 7, 2).forEach { code ->
            assertEquals(
                MediaError.AUTHENTICATION_FAILED,
                MediaErrorMapper.toMediaException(MediaLibException(code, "x")).error,
            )
        }
    }

    @Test
    fun `structural failures map to CORRUPTED`() {
        listOf(4, 5, 8, 9, 10, 11, 17).forEach { code ->
            assertEquals(MediaError.CORRUPTED, MediaErrorMapper.map(NativeErrorCode.fromCode(code)))
        }
    }

    @Test
    fun `unsupported container means metadata could not be removed`() {
        assertEquals(
            MediaError.METADATA_UNSUPPORTED,
            MediaErrorMapper.toMediaException(MediaLibException(16, "unsupported container: Ogg")).error,
        )
        assertEquals(MediaError.UNSUPPORTED_MEDIA, MediaErrorMapper.map(NativeErrorCode.CONTAINER_TOO_LARGE))
    }

    @Test
    fun `every native code has a mapping`() {
        NativeErrorCode.entries.forEach { MediaErrorMapper.map(it) }
    }

    @Test
    fun `io failures map to storage or not found`() {
        assertEquals(MediaError.STORAGE, MediaErrorMapper.map(NativeErrorCode.IO))
        assertEquals(MediaError.NOT_FOUND, MediaErrorMapper.toMediaException(FileNotFoundException()).error)
        assertEquals(MediaError.STORAGE, MediaErrorMapper.toMediaException(IOException()).error)
        assertEquals(MediaError.UNSUPPORTED_MEDIA, MediaErrorMapper.map(NativeErrorCode.UNSUPPORTED_MIME_TYPE))
    }

    @Test
    fun `mapMediaErrors rethrows as MediaException`() {
        val e = runCatching { mapMediaErrors { throw MediaLibException(7, "chunk 3 authentication failed") } }
            .exceptionOrNull() as org.anon.mediaplayer.domain.model.MediaException
        assertEquals(MediaError.AUTHENTICATION_FAILED, e.error)
        assertEquals("chunk 3 authentication failed", e.message)
    }
}
