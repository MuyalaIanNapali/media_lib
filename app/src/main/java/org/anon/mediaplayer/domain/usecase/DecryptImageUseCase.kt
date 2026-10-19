package org.anon.mediaplayer.domain.usecase

import javax.inject.Inject
import org.anon.mediaplayer.domain.repository.ImageDecryptor

class DecryptImageUseCase @Inject constructor(
    private val decryptor: ImageDecryptor,
) {
    suspend operator fun invoke(mediaId: Long): Result<ByteArray> =
        mediaResultOf { decryptor.decrypt(mediaId) }
}
