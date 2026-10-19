package org.anon.mediaplayer.ui.navigation

object Routes {
    const val ARG_MEDIA_ID = "mediaId"
    const val ARG_IS_AUDIO = "isAudio"

    const val ENCRYPT = "encrypt"
    const val SECURE_FOLDER = "secure_folder"
    const val IMAGE_VIEWER = "image/{$ARG_MEDIA_ID}"
    const val VIDEO_PLAYER = "video/{$ARG_MEDIA_ID}"
    const val AUDIO_PLAYER = "audio/{$ARG_MEDIA_ID}"

    fun imageViewer(mediaId: Long) = "image/$mediaId"
    fun videoPlayer(mediaId: Long) = "video/$mediaId"
    fun audioPlayer(mediaId: Long) = "audio/$mediaId"
}
