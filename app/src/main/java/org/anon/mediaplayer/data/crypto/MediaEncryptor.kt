package org.anon.mediaplayer.data.crypto

import android.content.ContentResolver
import android.net.Uri
import javax.inject.Inject
import org.anon.mediaplayer.data.local.EncryptedFileStore
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType

/** A freshly encrypted file and the key that opens it. */
class EncryptedFile(
    val fileName: String,
    val mimeType: String,
    val key: ByteArray,
)

/** Turns picked media into an encrypted file in the [EncryptedFileStore]. */
interface MediaEncryptor {
    /** Strips metadata and encrypts the media at [uri]. Blocking. */
    fun encrypt(uri: String): EncryptedFile
}

class NativeMediaEncryptor @Inject constructor(
    private val crypto: NativeMediaCrypto,
    private val store: EncryptedFileStore,
    private val contentResolver: ContentResolver,
) : MediaEncryptor {

    override fun encrypt(uri: String): EncryptedFile {
        val source = Uri.parse(uri)
        val mimeType = contentResolver.getType(source)
            ?: throw MediaException(MediaError.UNSUPPORTED_MEDIA, "Unknown type for $uri")
        MediaType.fromMimeType(mimeType)
            ?: throw MediaException(MediaError.UNSUPPORTED_MEDIA, mimeType)

        val outputDir = store.ensureDirectory()
        val fileName = store.newFileName()
        val key = mapMediaErrors {
            val descriptor = contentResolver.openFileDescriptor(source, "r")
                ?: throw MediaException(MediaError.NOT_FOUND, "Could not open $uri")
            // detachFd() hands ownership to Rust, which closes it.
            crypto.stripAndEncrypt(descriptor.detachFd(), mimeType, outputDir.absolutePath, fileName)
        }
        return EncryptedFile(fileName, mimeType, key)
    }
}
