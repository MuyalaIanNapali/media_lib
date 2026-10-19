package org.anon.mediaplayer.ui.navigation

import androidx.annotation.DrawableRes
import androidx.annotation.StringRes
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import org.anon.mediaplayer.R
import org.anon.mediaplayer.domain.model.MediaType
import org.anon.mediaplayer.ui.encrypt.EncryptRoute
import org.anon.mediaplayer.ui.securefolder.SecureFolderRoute
import org.anon.mediaplayer.ui.viewer.ImageViewerRoute
import org.anon.mediaplayer.ui.viewer.MediaPlayerRoute

private enum class TopLevelDestination(
    val route: String,
    @StringRes val label: Int,
    @DrawableRes val icon: Int,
) {
    ENCRYPT(Routes.ENCRYPT, R.string.tab_encrypt, R.drawable.ic_add),
    SECURE_FOLDER(Routes.SECURE_FOLDER, R.string.tab_secure_folder, R.drawable.ic_lock),
}

@Composable
fun AppNavHost(navController: NavHostController = rememberNavController()) {
    val backStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = backStackEntry?.destination?.route
    val showBottomBar = TopLevelDestination.entries.any { it.route == currentRoute }

    Scaffold(
        bottomBar = {
            if (showBottomBar) {
                NavigationBar {
                    TopLevelDestination.entries.forEach { destination ->
                        NavigationBarItem(
                            selected = currentRoute == destination.route,
                            onClick = { navController.navigateToTopLevel(destination.route) },
                            icon = { Icon(painterResource(destination.icon), contentDescription = null) },
                            label = { Text(stringResource(destination.label)) },
                        )
                    }
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = Routes.ENCRYPT,
            modifier = Modifier.padding(padding),
        ) {
            composable(Routes.ENCRYPT) {
                EncryptRoute(onOpenSecureFolder = { navController.navigateToTopLevel(Routes.SECURE_FOLDER) })
            }
            composable(Routes.SECURE_FOLDER) {
                SecureFolderRoute(onOpenMedia = { media ->
                    navController.navigate(
                        when (media.type) {
                            MediaType.IMAGE -> Routes.imageViewer(media.id)
                            MediaType.VIDEO -> Routes.videoPlayer(media.id)
                            MediaType.AUDIO -> Routes.audioPlayer(media.id)
                        }
                    )
                })
            }
            composable(Routes.IMAGE_VIEWER, arguments = listOf(mediaIdArgument())) {
                ImageViewerRoute(onBack = navController::popBackStack)
            }
            composable(Routes.VIDEO_PLAYER, arguments = listOf(mediaIdArgument(), isAudioArgument(false))) {
                MediaPlayerRoute(onBack = navController::popBackStack)
            }
            composable(Routes.AUDIO_PLAYER, arguments = listOf(mediaIdArgument(), isAudioArgument(true))) {
                MediaPlayerRoute(onBack = navController::popBackStack)
            }
        }
    }
}

private fun mediaIdArgument() = navArgument(Routes.ARG_MEDIA_ID) { type = NavType.LongType }

private fun isAudioArgument(isAudio: Boolean) = navArgument(Routes.ARG_IS_AUDIO) {
    type = NavType.BoolType
    defaultValue = isAudio
}

private fun NavHostController.navigateToTopLevel(route: String) = navigate(route) {
    popUpTo(graph.findStartDestination().id) { saveState = true }
    launchSingleTop = true
    restoreState = true
}
