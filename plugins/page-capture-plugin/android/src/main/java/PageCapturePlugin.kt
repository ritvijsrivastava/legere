// Renders a page in a real, off-layout WebView for capture — the Android
// half of `capture::render_linux`'s design (see that Rust module's docs
// for the rationale: engine-correct parsing/typography without a
// separate bundled browser). JavaScript is disabled for the same reason
// documented there: a captured page's own script would otherwise run for
// real during capture, including third-party analytics/tracking.
//
// Unlike WebKitGTK's `resource-load-started`/`WebResource.getData()` (a
// passive, after-the-fact observer of a request the engine already
// made), `WebViewClient.shouldInterceptRequest` is fundamentally an
// interception point: returning null lets the real request proceed but
// forfeits the bytes, while supplying a `WebResourceResponse` requires
// fetching the resource yourself first. This does that with a plain
// `HttpURLConnection` per resource and relays the bytes back into the
// response so the page still renders (and further discovery, e.g. a
// `@font-face` inside a CSS file, still happens) exactly as it would if
// the request had gone untouched.

package com.ritvijsrivastava.legere.pagecapture

import android.app.Activity
import android.util.Base64
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

// Debounce after `onPageFinished`, mirroring `render_linux`'s
// `QUIET_PERIOD_MS` — with JS disabled, nothing can kick off a *new*
// request after the ones the initial HTML/CSS parse already discovered,
// so this only needs to cover requests already in flight.
private const val QUIET_PERIOD_MS: Long = 250
// Absolute ceiling regardless of settle state, mirroring
// `render_linux`'s `MAX_WAIT_SECS`, so a page that never truly settles
// still returns something instead of hanging a capture forever.
private const val MAX_WAIT_SECS: Long = 30
private const val FETCH_TIMEOUT_MS = 15000

@InvokeArg
class CaptureArgs {
    lateinit var url: String
}

private class CapturedResource(val url: String, val contentType: String?, val bytes: ByteArray)

@TauriPlugin
class PageCapturePlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun capturePage(invoke: Invoke) {
        val args = invoke.parseArgs(CaptureArgs::class.java)
        val resources = ConcurrentHashMap<String, CapturedResource>()
        val doneLatch = CountDownLatch(1)
        val finalUrl = AtomicReference(args.url)

        activity.runOnUiThread {
            val webView = WebView(activity)
            webView.settings.javaScriptEnabled = false

            webView.webViewClient = object : WebViewClient() {
                // Runs off the UI thread (WebView's own resource-loader
                // thread pool), so the blocking `HttpURLConnection` call
                // below doesn't stall rendering of what's already parsed.
                override fun shouldInterceptRequest(
                    view: WebView,
                    request: WebResourceRequest
                ): WebResourceResponse? {
                    val requestUrl = request.url.toString()
                    if (!requestUrl.startsWith("http://") && !requestUrl.startsWith("https://")) {
                        return null
                    }
                    return try {
                        val connection = URL(requestUrl).openConnection() as HttpURLConnection
                        connection.instanceFollowRedirects = true
                        connection.connectTimeout = FETCH_TIMEOUT_MS
                        connection.readTimeout = FETCH_TIMEOUT_MS
                        connection.connect()
                        val bytes = connection.inputStream.use { it.readBytes() }
                        val contentType = connection.contentType
                        val mimeType = contentType?.substringBefore(';')?.trim()
                        resources[requestUrl] = CapturedResource(requestUrl, contentType, bytes)
                        WebResourceResponse(
                            mimeType ?: "application/octet-stream",
                            "utf-8",
                            bytes.inputStream()
                        )
                    } catch (e: Exception) {
                        // Graceful degradation, matching
                        // `wraith_assets`'s own stance: a single failed
                        // resource shouldn't fail the whole capture, just
                        // fall through to the (unobserved) default
                        // handling for this one reference.
                        null
                    }
                }

                override fun onPageFinished(view: WebView, url: String) {
                    finalUrl.set(url)
                    view.postDelayed({ doneLatch.countDown() }, QUIET_PERIOD_MS)
                }
            }

            webView.loadUrl(args.url)
        }

        doneLatch.await(MAX_WAIT_SECS, TimeUnit.SECONDS)

        val resourcesJson = JSArray()
        for (r in resources.values) {
            val obj = JSObject()
            obj.put("url", r.url)
            obj.put("content_type", r.contentType)
            obj.put("body_base64", Base64.encodeToString(r.bytes, Base64.NO_WRAP))
            resourcesJson.put(obj)
        }
        val result = JSObject()
        result.put("final_url", finalUrl.get())
        result.put("resources", resourcesJson)
        invoke.resolve(result)
    }
}
