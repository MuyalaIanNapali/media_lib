package org.anon.mediaplayer

/**
 * JNI bindings for the Rust `media_lib`. Every function throws
 * [MediaLibException] on failure. Use it through
 * `data.crypto.NativeMediaCrypto` rather than directly.
 */
object MediaLib {

    init {
        System.loadLibrary("media_lib")
    }

    /**
     * Encrypts the media behind [fd] (ownership passes to Rust, which closes
     * it) to `outputDir/fileName`. Returns the 32-byte media key.
     */
    external fun nativeStripAndEncrypt(
        fd: Int,
        mime: String,
        outputDir: String,
        fileName: String
    ): ByteArray

    /** Decrypts a whole image (secretstream) file into memory. */
    external fun nativeDecrypt(
        inputDir: String,
        inputName: String,
        key: ByteArray
    ): ByteArray

    /**
     * Authenticates and decrypts up to [length] plaintext bytes from
     * [position] of a video (AES-256-CTR) file, touching only the chunks that
     * cover that range.
     */
    external fun nativeReadMedia(
        inputDir: String,
        inputName: String,
        position: Long,
        length: Long,
        key: ByteArray
    ): ByteArray
}
