package org.anon.mediaplayer.data.repository

import android.content.ContentResolver
import androidx.core.net.toUri
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.repository.MediaTypeResolver
import javax.inject.Inject

class MediaTypeResolverImpl @Inject constructor(
    private val contentResolver: ContentResolver,
) : MediaTypeResolver {

    override fun resolve(uri: String): MediaType? =
        contentResolver.getType(uri.toUri())?.let(MediaType::fromMimeType)
}
