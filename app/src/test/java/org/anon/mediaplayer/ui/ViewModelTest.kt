package org.anon.mediaplayer.ui

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.anon.mediaplayer.domain.model.MediaError
import org.anon.mediaplayer.domain.model.MediaException
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.domain.model.SecureMedia
import org.anon.mediaplayer.domain.repository.MediaTypeResolver
import org.anon.mediaplayer.domain.repository.SecureMediaRepository
import org.anon.mediaplayer.domain.repository.ThumbnailProvider
import org.anon.mediaplayer.domain.usecase.GetThumbnailUseCase
import org.anon.mediaplayer.domain.usecase.ImportMediaUseCase
import org.anon.mediaplayer.domain.usecase.ObserveSecureMediaUseCase
import org.anon.mediaplayer.domain.usecase.ResolveMediaTypeUseCase
import org.anon.mediaplayer.ui.encrypt.EncryptViewModel
import org.anon.mediaplayer.ui.securefolder.SecureFolderUiState
import org.anon.mediaplayer.ui.securefolder.SecureFolderViewModel
import org.anon.mediaplayer.ui.securefolder.ThumbnailState
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ViewModelTest {

    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private class FakeRepository : SecureMediaRepository {
        val items = MutableStateFlow<List<SecureMedia>>(emptyList())
        var importResult: suspend () -> SecureMedia = { SecureMedia(1, MediaType.IMAGE, 0) }
        override fun observeAll() = items
        override suspend fun import(uri: String) = importResult()
    }

    private val resolver = object : MediaTypeResolver {
        override fun resolve(uri: String) = when {
            uri.endsWith("mp4") -> MediaType.VIDEO
            uri.endsWith("mp3") -> MediaType.AUDIO
            else -> MediaType.IMAGE
        }
    }

    private fun encryptViewModel(repository: FakeRepository) =
        EncryptViewModel(ResolveMediaTypeUseCase(resolver), ImportMediaUseCase(repository))

    @Test
    fun `encrypt moves through encrypting to encrypted`() = runTest(dispatcher) {
        val repository = FakeRepository()
        val gate = CompletableDeferred<Unit>()
        repository.importResult = { gate.await(); SecureMedia(1, MediaType.VIDEO, 0) }
        val vm = encryptViewModel(repository)

        vm.onMediaPicked("content://clip.mp4")
        assertEquals(MediaType.VIDEO, vm.state.value.mediaType)

        vm.encrypt()
        advanceUntilIdle()
        assertTrue(vm.state.value.isEncrypting)

        gate.complete(Unit)
        advanceUntilIdle()
        assertFalse(vm.state.value.isEncrypting)
        assertTrue(vm.state.value.isEncrypted)
        assertNull(vm.state.value.error)
    }

    @Test
    fun `encrypt failure exposes the media error`() = runTest(dispatcher) {
        val repository = FakeRepository()
        repository.importResult = { throw MediaException(MediaError.UNSUPPORTED_MEDIA) }
        val vm = encryptViewModel(repository)

        vm.onMediaPicked("content://photo.jpg")
        vm.encrypt()
        advanceUntilIdle()

        assertEquals(MediaError.UNSUPPORTED_MEDIA, vm.state.value.error)
        assertFalse(vm.state.value.isEncrypted)
    }

    @Test
    fun `audio selection is recognised and metadata errors surface`() = runTest(dispatcher) {
        val repository = FakeRepository()
        repository.importResult = { throw MediaException(MediaError.METADATA_UNSUPPORTED) }
        val vm = encryptViewModel(repository)

        vm.onMediaPicked("content://song.mp3")
        assertEquals(MediaType.AUDIO, vm.state.value.mediaType)

        vm.encrypt()
        advanceUntilIdle()
        assertEquals(MediaError.METADATA_UNSUPPORTED, vm.state.value.error)
    }

    @Test
    fun `clear resets state`() = runTest(dispatcher) {
        val vm = encryptViewModel(FakeRepository())
        vm.onMediaPicked("content://photo.jpg")
        vm.clear()
        assertNull(vm.state.value.selectedUri)
    }

    @Test
    fun `secure folder lists items and loads each thumbnail once`() = runTest(dispatcher) {
        val repository = FakeRepository()
        var calls = 0
        val thumbnails = object : ThumbnailProvider {
            override suspend fun thumbnail(mediaId: Long): ByteArray {
                calls++
                if (mediaId == 2L) throw MediaException(MediaError.AUTHENTICATION_FAILED)
                return byteArrayOf(1)
            }
        }
        val vm = SecureFolderViewModel(ObserveSecureMediaUseCase(repository), GetThumbnailUseCase(thumbnails))
        val collector = launch { vm.state.collect {} }

        repository.items.value = listOf(SecureMedia(1, MediaType.IMAGE, 0), SecureMedia(2, MediaType.VIDEO, 0))
        advanceUntilIdle()
        val content = vm.state.first() as SecureFolderUiState.Content
        assertEquals(listOf(1L, 2L), content.items.map { it.id })

        vm.loadThumbnail(1)
        vm.loadThumbnail(1)
        vm.loadThumbnail(2)
        advanceUntilIdle()

        assertEquals(2, calls)
        assertTrue(vm.thumbnails.value[1] is ThumbnailState.Ready)
        assertEquals(ThumbnailState.Failed(MediaError.AUTHENTICATION_FAILED), vm.thumbnails.value[2])
        collector.cancel()
    }
}
