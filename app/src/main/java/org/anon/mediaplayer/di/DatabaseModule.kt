package org.anon.mediaplayer.di

import android.content.Context
import androidx.room.Room
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton
import org.anon.mediaplayer.data.local.MediaDao
import org.anon.mediaplayer.data.local.MediaDatabase

@Module
@InstallIn(SingletonComponent::class)
object DatabaseModule {

    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): MediaDatabase =
        Room.databaseBuilder(context, MediaDatabase::class.java, MediaDatabase.NAME)
            // v1 never stored rows (the insert path was unwired), so nothing is lost.
            .fallbackToDestructiveMigration(dropAllTables = true)
            .build()

    @Provides
    fun provideMediaDao(database: MediaDatabase): MediaDao = database.mediaDao()
}
