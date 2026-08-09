package aero.updraft

import android.os.Bundle
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature

class MainActivity : TauriActivity() {
  private var webView: WebView? = null
  private val safeAreaBridge = SafeAreaBridge()

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView
    webView.addJavascriptInterface(safeAreaBridge, "__updraftSafeArea")
    if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
      WebViewCompat.addDocumentStartJavaScript(webView, SAFE_AREA_JAVASCRIPT, setOf("*"))
    }

    ViewCompat.setOnApplyWindowInsetsListener(webView) { _, windowInsets ->
      val insets =
        windowInsets.getInsets(
          WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
        )
      val density = resources.displayMetrics.density
      safeAreaBridge.insets =
        SafeAreaInsets(
          top = insets.top / density,
          right = insets.right / density,
          bottom = insets.bottom / density,
          left = insets.left / density,
        )
      applySafeAreaInsets()
      windowInsets
    }
    ViewCompat.requestApplyInsets(webView)
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) applySafeAreaInsets()
  }

  private fun applySafeAreaInsets() {
    webView?.evaluateJavascript("window.__updraftApplySafeArea?.()", null)
  }

  private companion object {
    val SAFE_AREA_JAVASCRIPT =
      """
      window.__updraftApplySafeArea = () => {
        const root = document.documentElement
        if (!root) return
        root.style.setProperty('--safe-area-top', `${'$'}{window.__updraftSafeArea.top()}px`)
        root.style.setProperty('--safe-area-right', `${'$'}{window.__updraftSafeArea.right()}px`)
        root.style.setProperty('--safe-area-bottom', `${'$'}{window.__updraftSafeArea.bottom()}px`)
        root.style.setProperty('--safe-area-left', `${'$'}{window.__updraftSafeArea.left()}px`)
      }
      window.__updraftApplySafeArea()
      """.trimIndent()
  }
}

private data class SafeAreaInsets(
  val top: Float = 0f,
  val right: Float = 0f,
  val bottom: Float = 0f,
  val left: Float = 0f,
)

private class SafeAreaBridge {
  @Volatile var insets = SafeAreaInsets()

  @JavascriptInterface fun top() = insets.top

  @JavascriptInterface fun right() = insets.right

  @JavascriptInterface fun bottom() = insets.bottom

  @JavascriptInterface fun left() = insets.left
}
