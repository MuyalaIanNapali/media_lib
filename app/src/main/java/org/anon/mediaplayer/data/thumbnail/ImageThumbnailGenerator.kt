package org.anon.mediaplayer.data.thumbnail

import android.graphics.Bitmap
import android.graphics.ImageDecoder
import java.nio.ByteBuffer
import javax.inject.Inject
import org.anon.mediaplayer.data.crypto.EncryptedImageReader
import org.anon.mediaplayer.data.local.EncryptedMediaRef
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType

/** Images are small enough to decrypt whole, in memory, then downscale. */
class ImageThumbnailGenerator @Inject constructor(
    private val reader: EncryptedImageReader,
) : ThumbnailGenerator {

    override val type = MediaType.IMAGE

    override fun generate(ref: EncryptedMediaRef, maxSize: Int): Bitmap {
        val bytes = reader.read(ref)
        return try {
            ImageDecoder.decodeBitmap(ImageDecoder.createSource(ByteBuffer.wrap(bytes))) { decoder, info, _ ->
                val longest = maxOf(info.size.width, info.size.height)
                if (longest > maxSize) {
                    val scale = maxSize.toFloat() / longest
                    decoder.setTargetSize(
                        (info.size.width * scale).toInt().coerceAtLeast(1),
                        (info.size.height * scale).toInt().coerceAtLeast(1),
                    )
                }
                // Software bitmaps can be re-encoded to JPEG.
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            }
        } catch (e: Exception) {
            throw MediaException(MediaError.UNSUPPORTED_MEDIA, "Image can't be decoded", e)
        }
    }
}
