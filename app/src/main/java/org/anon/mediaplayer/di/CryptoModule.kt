package org.anon.mediaplayer.di

import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import org.anon.mediaplayer.data.crypto.EncryptedImageReader
import org.anon.mediaplayer.data.crypto.MediaEncryptor
import org.anon.mediaplayer.data.crypto.MediaLibNativeCrypto
import org.anon.mediaplayer.data.crypto.NativeImageReader
import org.anon.mediaplayer.data.crypto.NativeMediaCrypto
import org.anon.mediaplayer.data.crypto.NativeMediaEncryptor
import org.anon.mediaplayer.data.video.NativePlaintextReaderFactory
import org.anon.mediaplayer.data.video.PlaintextReaderFactory

/** Encryption and decryption, all backed by the native library. */
@Module
@InstallIn(SingletonComponent::class)
abstract class CryptoModule {
    @Binds
    abstract fun bindNativeMediaCrypto(impl: MediaLibNativeCrypto): NativeMediaCrypto

    @Binds
    abstract fun bindMediaEncryptor(impl: NativeMediaEncryptor): MediaEncryptor

    @Binds
    abstract fun bindEncryptedImageReader(impl: NativeImageReader): EncryptedImageReader

    @Binds
    abstract fun bindPlaintextReaderFactory(impl: NativePlaintextReaderFactory): PlaintextReaderFactory
}
