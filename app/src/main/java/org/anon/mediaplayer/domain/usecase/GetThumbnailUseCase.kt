package org.anon.mediaplayer.domain.usecase

import javax.inject.Inject
import org.anon.mediaplayer.domain.repository.ThumbnailProvider

class GetThumbnailUseCase @Inject constructor(
    private val provider: ThumbnailProvider,
) {
    suspend operator fun invoke(mediaId: Long): Result<ByteArray> =
        mediaResultOf { provider.thumbnail(mediaId) }
}
