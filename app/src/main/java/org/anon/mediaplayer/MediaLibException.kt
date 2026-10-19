package org.anon.mediaplayer

/**
 * Thrown by every `MediaLib` native call. [code] is the Rust
 * `MediaLibError::code()`; see `data.crypto.NativeErrorCode`.
 *
 * Constructed from Rust via JNI with signature `(ILjava/lang/String;)V`, so
 * the class name and constructor must not change (and are kept from R8).
 */
class MediaLibException(
    val code: Int,
    message: String,
) : RuntimeException(message)
