package org.anon.mediaplayer.data.local

import java.io.File
import org.anon.mediaplayer.domain.model.MediaType

/** Everything needed to decrypt one item. Never leaves the data layer. */
class EncryptedMediaRef(
    val id: Long,
    val type: MediaType,
    val file: File,
    val key: ByteArray,
) {
    val directory: String get() = file.parent.orEmpty()
    val fileName: String get() = file.name
}
