package org.anon.mediaplayer.domain.repository

import org.anon.mediaplayer.domain.model.MediaType

interface MediaTypeResolver {
    /** The type of the (not yet encrypted) media at [uri], or null if unsupported. */
    fun resolve(uri: String): MediaType?
}
