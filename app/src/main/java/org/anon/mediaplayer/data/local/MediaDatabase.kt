package org.anon.mediaplayer.data.local

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(entities = [MediaEntity::class], version = 2, exportSchema = false)
abstract class MediaDatabase : RoomDatabase() {
    abstract fun mediaDao(): MediaDao

    companion object {
        const val NAME = "secure_media.db"
    }
}
