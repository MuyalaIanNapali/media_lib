package org.anon.mediaplayer.di

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import org.anon.mediaplayer.data.repository.ImageDecryptorImpl
import org.anon.mediaplayer.data.repository.MediaTypeResolverImpl
import org.anon.mediaplayer.data.repository.SecureMediaRepositoryImpl
import org.anon.mediaplayer.data.thumbnail.ThumbnailProviderImpl
import org.anon.mediaplayer.domain.repository.ImageDecryptor
import org.anon.mediaplayer.domain.repository.MediaTypeResolver
import org.anon.mediaplayer.domain.repository.SecureMediaRepository
import org.anon.mediaplayer.domain.repository.ThumbnailProvider

/** Implementations of the domain layer's interfaces. */
@Module
@InstallIn(SingletonComponent::class)
abstract class RepositoryModule {
    @Binds
    abstract fun bindSecureMediaRepository(impl: SecureMediaRepositoryImpl): SecureMediaRepository

    @Binds
    abstract fun bindMediaTypeResolver(impl: MediaTypeResolverImpl): MediaTypeResolver

    @Binds
    abstract fun bindImageDecryptor(impl: ImageDecryptorImpl): ImageDecryptor

    @Binds
    abstract fun bindThumbnailProvider(impl: ThumbnailProviderImpl): ThumbnailProvider
}
