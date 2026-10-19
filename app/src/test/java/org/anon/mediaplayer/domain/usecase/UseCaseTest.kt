package org.anon.mediaplayer.domain.usecase

import kotlinx.coroutines.test.runTest
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.repository.ImageDecryptor
import org.anon.mediaplayer.domain.repository.ThumbnailProvider
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test

class UseCaseTest {

    @Test
    fun `decrypt image success is wrapped`() = runTest {
        val bytes = byteArrayOf(1, 2, 3)
        val result = DecryptImageUseCase(ImageDecryptor { bytes })(7)
        assertArrayEquals(bytes, result.getOrThrow())
    }

    @Test
    fun `media exception keeps its error`() = runTest {
        val useCase = GetThumbnailUseCase(ThumbnailProvider {
            throw MediaException(MediaError.AUTHENTICATION_FAILED)
        })
        assertEquals(MediaError.AUTHENTICATION_FAILED, useCase(1).exceptionOrNull()!!.mediaError)
    }

    @Test
    fun `unexpected exception becomes UNKNOWN`() = runTest {
        val useCase = GetThumbnailUseCase(ThumbnailProvider { error("boom") })
        val failure = useCase(1).exceptionOrNull()
        assertEquals(MediaError.UNKNOWN, failure!!.mediaError)
        assertEquals("boom", failure.cause!!.message)
    }

    private fun ImageDecryptor(block: suspend (Long) -> ByteArray) = object : ImageDecryptor {
        override suspend fun decrypt(mediaId: Long) = block(mediaId)
    }

    private fun ThumbnailProvider(block: suspend (Long) -> ByteArray) = object : ThumbnailProvider {
        override suspend fun thumbnail(mediaId: Long) = block(mediaId)
    }
}
