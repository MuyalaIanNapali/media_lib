package org.anon.mediaplayer.data.repository

import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import org.anon.mediaplayer.data.crypto.MediaEncryptor
import org.anon.mediaplayer.data.local.EncryptedFileStore
import org.anon.mediaplayer.data.local.MediaDao
import org.anon.mediaplayer.data.local.MediaEntity
import org.anon.mediaplayer.di.IoDispatcher
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.model.SecureMedia
import org.anon.mediaplayer.domain.repository.SecureMediaRepository

/** Records encrypted media; the encryption itself is the [MediaEncryptor]'s job. */
class SecureMediaRepositoryImpl @Inject constructor(
    private val dao: MediaDao,
    private val encryptor: MediaEncryptor,
    private val store: EncryptedFileStore,
    @IoDispatcher private val io: CoroutineDispatcher,
) : SecureMediaRepository {

    override fun observeAll(): Flow<List<SecureMedia>> =
        dao.observeAll().map { entities -> entities.mapNotNull { it.toDomain() } }

    override suspend fun import(uri: String): SecureMedia = withContext(io) {
        val encrypted = encryptor.encrypt(uri)
        val entity = MediaEntity(
            fileName = encrypted.fileName,
            mimeType = encrypted.mimeType,
            key = encrypted.key,
            createdAt = System.currentTimeMillis(),
        )

        val id = try {
            dao.insert(entity)
        } catch (e: Exception) {
            // Without its row the file is undecryptable; don't leave it behind.
            store.delete(encrypted.fileName)
            throw MediaException(MediaError.STORAGE, "Saving media failed", e)
        }

        entity.copy(id = id).toDomain()
            ?: throw MediaException(MediaError.UNSUPPORTED_MEDIA, encrypted.mimeType)
    }

    private fun MediaEntity.toDomain(): SecureMedia? =
        MediaType.fromMimeType(mimeType)?.let { SecureMedia(id, it, createdAt) }
}
