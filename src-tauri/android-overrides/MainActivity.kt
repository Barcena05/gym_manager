package com.gym_manager.dc

import android.os.Build
import android.os.Bundle
import android.util.Log
import android.webkit.WebSettings
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import java.io.File

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Use a different WebView profile for each app version. Android WebView
    // can otherwise keep serving the old index.html/chunks after an APK
    // update. This must run before Tauri creates the WebView.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      WebView.setDataDirectorySuffix("gymmanager-${BuildConfig.VERSION_NAME}")
    }

    // Fallback for older Android versions, and also clean the old default
    // profile from previous versions. This does not touch the app's SQLite
    // database or settings.
    clearWebViewData()

    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)

    // Second line of defence for the current process.
    webView.clearCache(true)
    webView.clearHistory()
    webView.settings.cacheMode = WebSettings.LOAD_NO_CACHE
  }

  private fun clearWebViewData() {
    try {
      val webviewDir = File(applicationInfo.dataDir, "app_webview")
      if (webviewDir.exists()) {
        val deleted = webviewDir.deleteRecursively()
        Log.i("MainActivity", "Cleared WebView data: $deleted")
      }

      val webviewCache = File(cacheDir, "WebView")
      if (webviewCache.exists()) {
        webviewCache.deleteRecursively()
      }
    } catch (e: Exception) {
      Log.w("MainActivity", "Could not clear WebView data", e)
    }
  }
}
