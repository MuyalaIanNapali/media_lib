package org.anon.mediaplayer.data.local

import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import org.anon.mediaplayer.di.IoDispatcher
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType

/** Resolves a media id to its encrypted file and key. */
interface MediaLocator {
    /**
     * Fails with [MediaError.NOT_FOUND] if the item or its file is gone, and
     * [MediaError.UNSUPPORTED_MEDIA] if its type isn't in [expectedTypes].
     */
    suspend fun locate(mediaId: Long, expectedTypes: Set<MediaType> = MediaType.entries.toSet()): EncryptedMediaRef
}

class RoomMediaLocator @Inject constructor(
    private val dao: MediaDao,
    private val store: EncryptedFileStore,
    @IoDispatcher private val io: CoroutineDispatcher,
) : MediaLocator {

    override suspend fun locate(mediaId: Long, expectedTypes: Set<MediaType>): EncryptedMediaRef =
        withContext(io) {
            val entity = dao.getById(mediaId)
                ?: throw MediaException(MediaError.NOT_FOUND, "No media with id $mediaId")

            val type = MediaType.fromMimeType(entity.mimeType)
                ?: throw MediaException(MediaError.UNSUPPORTED_MEDIA, entity.mimeType)
            if (type !in expectedTypes) {
                throw MediaException(MediaError.UNSUPPORTED_MEDIA, "Media $mediaId is $type")
            }

            val file = store.file(entity.fileName)
            if (!file.exists()) {
                throw MediaException(MediaError.NOT_FOUND, "Encrypted file missing for $mediaId")
            }

            EncryptedMediaRef(mediaId, type, file, entity.key)
        }
}
