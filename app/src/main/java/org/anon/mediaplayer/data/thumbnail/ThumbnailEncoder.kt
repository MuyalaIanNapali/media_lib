package org.anon.mediaplayer.data.thumbnail

import android.graphics.Bitmap
import java.io.ByteArrayOutputStream
import javax.inject.Inject

/** Encodes a preview frame into bytes the UI can display. */
interface ThumbnailEncoder {
    /** Encodes [bitmap] and recycles it. */
    fun encode(bitmap: Bitmap): ByteArray
}

class JpegThumbnailEncoder @Inject constructor() : ThumbnailEncoder {

    override fun encode(bitmap: Bitmap): ByteArray = try {
        ByteArrayOutputStream().use { out ->
            bitmap.compress(Bitmap.CompressFormat.JPEG, QUALITY, out)
            out.toByteArray()
        }
    } finally {
        bitmap.recycle()
    }

    private companion object {
        const val QUALITY = 85
    }
}
