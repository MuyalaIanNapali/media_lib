package org.anon.mediaplayer.domain.model

/**
 * An encrypted media item in the secure folder. Deliberately carries no key
 * or file path: those never leave the data layer.
 */
data class SecureMedia(
    val id: Long,
    val type: MediaType,
    val createdAt: Long,
)

enum class MediaType {
    IMAGE,
    VIDEO,
    AUDIO;

    companion object {
        fun fromMimeType(mimeType: String): MediaType? = when {
            mimeType.startsWith("image/") -> IMAGE
            mimeType.startsWith("video/") -> VIDEO
            mimeType.startsWith("audio/") -> AUDIO
            else -> null
        }
    }
}
