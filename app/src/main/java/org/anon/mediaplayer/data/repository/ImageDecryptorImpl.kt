package org.anon.mediaplayer.data.repository

import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import org.anon.mediaplayer.data.crypto.EncryptedImageReader
import org.anon.mediaplayer.data.local.MediaLocator
import org.anon.mediaplayer.di.IoDispatcher
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.repository.ImageDecryptor

class ImageDecryptorImpl @Inject constructor(
    private val locator: MediaLocator,
    private val reader: EncryptedImageReader,
    @IoDispatcher private val io: CoroutineDispatcher,
) : ImageDecryptor {

    override suspend fun decrypt(mediaId: Long): ByteArray {
        val ref = locator.locate(mediaId, expectedTypes = setOf(MediaType.IMAGE))
        return withContext(io) { reader.read(ref) }
    }
}
