package org.anon.mediaplayer.di

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import org.anon.mediaplayer.playback.ExoMediaPlayerFactory
import org.anon.mediaplayer.playback.MediaPlayerFactory

@Module
@InstallIn(SingletonComponent::class)
abstract class PlaybackModule {
    @Binds
    abstract fun bindMediaPlayerFactory(impl: ExoMediaPlayerFactory): MediaPlayerFactory
}
