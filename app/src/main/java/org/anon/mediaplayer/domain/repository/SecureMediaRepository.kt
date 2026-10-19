package org.anon.mediaplayer.domain.repository

import kotlinx.coroutines.flow.Flow
import org.anon.mediaplayer.domain.model.SecureMedia

interface SecureMediaRepository {
    fun observeAll(): Flow<List<SecureMedia>>

    /** Encrypts the media at [uri] into the secure folder. */
    suspend fun import(uri: String): SecureMedia
}
