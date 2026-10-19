package org.anon.mediaplayer.domain.usecase

import javax.inject.Inject
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.repository.MediaTypeResolver

class ResolveMediaTypeUseCase @Inject constructor(
    private val resolver: MediaTypeResolver,
) {
    operator fun invoke(uri: String): MediaType? = resolver.resolve(uri)
}
