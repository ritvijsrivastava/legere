package com.ritvijsrivastava.legere

import android.os.Bundle
import android.view.textclassifier.TextClassifier
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // Hands the JVM/Context to rustls-platform-verifier before any TLS
  // handshake can occur — see mobile_tls.rs for why this can't be done
  // from Rust's own startup path on Android. liblegere_lib.so is already
  // loaded by this point (Rust.kt's System.loadLibrary runs earlier in
  // this same onCreate chain, via the TauriActivity/WryActivity base).
  private external fun initTls()

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    initTls()
  }

  // Every <input>/<textarea> in the WebView is backed by a real Android
  // EditText, which by default runs the system TextClassifier (on-device
  // entity detection for phone numbers/addresses/etc., feeding autofill
  // and "smart" text selection) on each edit. That classifier is known to
  // be pathologically slow on digit-heavy input and phone-dialing
  // punctuation ('*', '#') in particular, freezing the whole WebView—
  // including unrelated keystrokes like Backspace — for seconds at a
  // time (this is what made the library search box hang when searching
  // numbers or '*'). Legere has no use for on-device entity detection
  // anywhere in its UI, so disable it outright rather than trying to
  // dodge it per-input.
  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.textClassifier = TextClassifier.NO_OP
  }
}
