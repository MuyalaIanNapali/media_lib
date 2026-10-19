package org.anon.mediaplayer.playback

import android.content.Context
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import org.anon.mediaplayer.data.crypto.mapMediaErrors
import org.anon.mediaplayer.data.local.MediaLocator
import org.anon.mediaplayer.data.video.PlaintextReaderFactory
import org.anon.mediaplayer.di.IoDispatcher
import org.anon.mediaplayer.domain.model.MediaType

/**
 * ExoPlayer over an [EncryptedDataSource]. Video and audio share the CHATMED2
 * format, so one chunked reader serves both.
 */
class ExoMediaPlayerFactory @Inject constructor(
    @ApplicationContext private val context: Context,
    private val locator: MediaLocator,
    private val readerFactory: PlaintextReaderFactory,
    @IoDispatcher private val io: CoroutineDispatcher,
) : MediaPlayerFactory {

    @OptIn(UnstableApi::class)
    override suspend fun create(mediaId: Long): Player {
        val ref = locator.locate(mediaId, expectedTypes = setOf(MediaType.VIDEO, MediaType.AUDIO))
        val reader = withContext(io) { mapMediaErrors { readerFactory.create(ref) } }

        val mediaSource = ProgressiveMediaSource.Factory(EncryptedDataSource.Factory(reader))
            .createMediaSource(MediaItem.fromUri("secure://media/$mediaId"))

        return ExoPlayer.Builder(context)
            .setLoadControl(PlaybackBufferPolicy.loadControl())
            .build()
            .apply {
                setMediaSource(mediaSource)
                prepare()
            }
    }
}
