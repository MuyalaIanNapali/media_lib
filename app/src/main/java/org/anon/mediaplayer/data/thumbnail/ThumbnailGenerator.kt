package org.anon.mediaplayer.data.thumbnail

import android.graphics.Bitmap
import org.anon.mediaplayer.data.local.EncryptedMediaRef
import org.anon.mediaplayer.domain.model.MediaType

/** Produces a preview frame for one [MediaType]. Blocking; call off the main thread. */
interface ThumbnailGenerator {
    val type: MediaType

    /** A bitmap whose longest side is at most [maxSize] px. */
    fun generate(ref: EncryptedMediaRef, maxSize: Int): Bitmap
}
