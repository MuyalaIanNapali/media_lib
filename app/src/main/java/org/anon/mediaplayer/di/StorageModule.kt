package org.anon.mediaplayer.di

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import org.anon.mediaplayer.data.local.AppPrivateFileStore
import org.anon.mediaplayer.data.local.EncryptedFileStore
import org.anon.mediaplayer.data.local.MediaLocator
import org.anon.mediaplayer.data.local.RoomMediaLocator

/** Where encrypted files and their records live. */
@Module
@InstallIn(SingletonComponent::class)
abstract class StorageModule {
    @Binds
    abstract fun bindEncryptedFileStore(impl: AppPrivateFileStore): EncryptedFileStore

    @Binds
    abstract fun bindMediaLocator(impl: RoomMediaLocator): MediaLocator
}
