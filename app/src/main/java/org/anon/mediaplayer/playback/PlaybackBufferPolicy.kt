package org.anon.mediaplayer.playback

import androidx.annotation.OptIn
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.LoadControl

/**
 * Caps how much decrypted media ExoPlayer holds in memory.
 *
 * Everything ExoPlayer buffers has already passed through
 * [EncryptedDataSource], so its buffer is plaintext. The default load control
 * keeps up to 50 s (and up to ~130 MB for video); this keeps a few seconds
 * and at most [TARGET_BUFFER_BYTES], with no back buffer.
 */
object PlaybackBufferPolicy {
    const val MIN_BUFFER_MS = 3_000
    const val MAX_BUFFER_MS = 10_000
    const val BUFFER_FOR_PLAYBACK_MS = 1_000
    const val BUFFER_FOR_PLAYBACK_AFTER_REBUFFER_MS = 2_000
    const val TARGET_BUFFER_BYTES = 8 * 1024 * 1024

    @OptIn(UnstableApi::class)
    fun loadControl(): LoadControl = DefaultLoadControl.Builder()
        .setBufferDurationsMs(
            MIN_BUFFER_MS,
            MAX_BUFFER_MS,
            BUFFER_FOR_PLAYBACK_MS,
            BUFFER_FOR_PLAYBACK_AFTER_REBUFFER_MS,
        )
        .setTargetBufferBytes(TARGET_BUFFER_BYTES)
        // Stop loading at the byte cap even if below the minimum duration.
        .setPrioritizeTimeOverSizeThresholds(false)
        .setBackBuffer(/* backBufferDurationMs = */ 0, /* retainBackBufferFromKeyframe = */ false)
        .build()
}
