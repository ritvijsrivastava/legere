package com.ritvijsrivastava.legere

import android.os.Bundle
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
}
