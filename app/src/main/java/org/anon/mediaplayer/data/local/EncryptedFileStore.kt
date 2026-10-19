package org.anon.mediaplayer.data.local

import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException

/** Where encrypted media files live. */
interface EncryptedFileStore {
    /** The directory for encrypted files, created if needed. */
    fun ensureDirectory(): File

    fun newFileName(): String

    fun file(fileName: String): File

    fun delete(fileName: String)
}

/** Encrypted files in app-private storage (`filesDir/encrypted_media`). */
@Singleton
class AppPrivateFileStore @Inject constructor(
    @ApplicationContext context: Context,
) : EncryptedFileStore {

    private val directory = File(context.filesDir, "encrypted_media")

    override fun ensureDirectory(): File {
        if (!directory.exists() && !directory.mkdirs()) {
            throw MediaException(MediaError.STORAGE, "Could not create ${directory.name}")
        }
        return directory
    }

    override fun newFileName(): String = "${UUID.randomUUID()}.enc"

    override fun file(fileName: String): File = File(directory, fileName)

    override fun delete(fileName: String) {
        file(fileName).delete()
    }
}
