package org.anon.mediaplayer.di

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import org.anon.mediaplayer.data.thumbnail.ImageThumbnailGenerator
import org.anon.mediaplayer.data.thumbnail.JpegThumbnailEncoder
import org.anon.mediaplayer.data.thumbnail.LruThumbnailCache
import org.anon.mediaplayer.data.thumbnail.ThumbnailCache
import org.anon.mediaplayer.data.thumbnail.ThumbnailEncoder
import org.anon.mediaplayer.data.thumbnail.ThumbnailGenerator
import org.anon.mediaplayer.data.thumbnail.VideoThumbnailGenerator

/** Thumbnail generation. Add a media type by binding another generator here. */
@Module
@InstallIn(SingletonComponent::class)
abstract class ThumbnailModule {
    @Binds
    @IntoSet
    abstract fun bindImageThumbnailGenerator(impl: ImageThumbnailGenerator): ThumbnailGenerator

    @Binds
    @IntoSet
    abstract fun bindVideoThumbnailGenerator(impl: VideoThumbnailGenerator): ThumbnailGenerator

    @Binds
    abstract fun bindThumbnailCache(impl: LruThumbnailCache): ThumbnailCache

    @Binds
    abstract fun bindThumbnailEncoder(impl: JpegThumbnailEncoder): ThumbnailEncoder
}
