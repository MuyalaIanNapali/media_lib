package org.anon.mediaplayer.data.crypto

/**
 * Mirror of the Rust `MediaLibError::code()` values (media_lib/src/error.rs).
 * Codes are append-only on both sides.
 */
enum class NativeErrorCode(val code: Int) {
    IO(1),
    INVALID_KEY_LENGTH(2),
    UNSUPPORTED_MIME_TYPE(3),
    INVALID_HEADER(4),
    UNSUPPORTED_FORMAT(5),
    HEADER_AUTHENTICATION_FAILED(6),
    CHUNK_AUTHENTICATION_FAILED(7),
    TRUNCATED(8),
    TRAILING_DATA(9),
    INVALID_CHUNK_LENGTH(10),
    FILE_SIZE_MISMATCH(11),
    INVALID_RANGE(12),
    KEY_DERIVATION(13),
    ENCRYPTION_FAILED(14),
    JNI(15),
    UNSUPPORTED_CONTAINER(16),
    MALFORMED_CONTAINER(17),
    CONTAINER_TOO_LARGE(18),
    UNKNOWN(-1);

    companion object {
        private val byCode = entries.associateBy(NativeErrorCode::code)

        fun fromCode(code: Int): NativeErrorCode = byCode[code] ?: UNKNOWN
    }
}
