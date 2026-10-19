package org.anon.mediaplayer.domain.usecase

import javax.inject.Inject
import kotlinx.coroutines.flow.Flow
import org.anon.mediaplayer.domain.model.SecureMedia
import org.anon.mediaplayer.domain.repository.SecureMediaRepository

class ObserveSecureMediaUseCase @Inject constructor(
    private val repository: SecureMediaRepository,
) {
    operator fun invoke(): Flow<List<SecureMedia>> = repository.observeAll()
}
