package org.anon.mediaplayer.domain.model

/** Failures the UI can explain to the user, independent of where they came from. */
enum class MediaError {
    /** Wrong key, or the encrypted file or header was tampered with. */
    AUTHENTICATION_FAILED,

    /** The encrypted file is truncated, padded or structurally invalid. */
    CORRUPTED,

    /** The media type or container can't be encrypted or played. */
    UNSUPPORTED_MEDIA,

    /** The format's metadata can't be removed, so it was refused. */
    METADATA_UNSUPPORTED,

    /** The media item or its encrypted file no longer exists. */
    NOT_FOUND,

    /** Reading or writing storage failed. */
    STORAGE,

    UNKNOWN,
}

class MediaException(
    val error: MediaError,
    message: String? = null,
    cause: Throwable? = null,
) : Exception(message ?: error.name, cause)
