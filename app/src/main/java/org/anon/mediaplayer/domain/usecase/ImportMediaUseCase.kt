package org.anon.mediaplayer.domain.usecase

import javax.inject.Inject
import org.anon.mediaplayer.domain.model.SecureMedia
import org.anon.mediaplayer.domain.repository.SecureMediaRepository

class ImportMediaUseCase @Inject constructor(
    private val repository: SecureMediaRepository,
) {
    suspend operator fun invoke(uri: String): Result<SecureMedia> =
        mediaResultOf { repository.import(uri) }
}
