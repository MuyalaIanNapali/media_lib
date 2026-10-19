package org.anon.mediaplayer.data.local

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

@Dao
interface MediaDao {
    @Insert
    suspend fun insert(media: MediaEntity): Long

    @Query("SELECT * FROM media WHERE id = :id")
    suspend fun getById(id: Long): MediaEntity?

    @Query("SELECT * FROM media ORDER BY createdAt DESC")
    fun observeAll(): Flow<List<MediaEntity>>

    @Query("DELETE FROM media WHERE id = :id")
    suspend fun deleteById(id: Long)
}
