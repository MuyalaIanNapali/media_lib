package org.anon.mediaplayer.playback

import androidx.media3.common.Player

/** Creates players for encrypted media; the UI's only playback dependency. */
interface MediaPlayerFactory {
    /** A prepared player for an encrypted video or audio item. Call on the main thread. */
    suspend fun create(mediaId: Long): Player
}
