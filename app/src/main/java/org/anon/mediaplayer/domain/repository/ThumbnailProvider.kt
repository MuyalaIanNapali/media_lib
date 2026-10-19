package org.anon.mediaplayer.domain.repository

interface ThumbnailProvider {
    /** A small JPEG preview of the media, generated in memory. */
    suspend fun thumbnail(mediaId: Long): ByteArray
}
