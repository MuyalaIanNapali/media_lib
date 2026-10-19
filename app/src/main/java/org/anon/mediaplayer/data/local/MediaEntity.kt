package org.anon.mediaplayer.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "media")
data class MediaEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    /** Name of the encrypted file in the [EncryptedFileStore]. */
    val fileName: String,
    val mimeType: String,
    /** The 32-byte media key returned by `nativeStripAndEncrypt`. */
    val key: ByteArray,
    val createdAt: Long,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as MediaEntity

        if (id != other.id) return false
        if (fileName != other.fileName) return false
        if (mimeType != other.mimeType) return false
        if (!key.contentEquals(other.key)) return false
        if (createdAt != other.createdAt) return false

        return true
    }

    override fun hashCode(): Int {
        var result = id.hashCode()
        result = 31 * result + fileName.hashCode()
        result = 31 * result + mimeType.hashCode()
        result = 31 * result + key.contentHashCode()
        result = 31 * result + createdAt.hashCode()
        return result
    }
}
