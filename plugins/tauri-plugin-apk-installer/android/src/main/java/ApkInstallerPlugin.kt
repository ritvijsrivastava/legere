package com.ritvijsrivastava.legere.apkinstaller

import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class InstallArgs {
  lateinit var path: String
}

@TauriPlugin
class ApkInstallerPlugin(private val activity: Activity) : Plugin(activity) {

  @Command
  fun canRequestInstalls(invoke: Invoke) {
    val result = JSObject()
    result.put("value", canInstallPackages())
    invoke.resolve(result)
  }

  /**
   * REQUEST_INSTALL_PACKAGES isn't a normal runtime permission — there's no
   * system dialog for it. The only way to grant it is the per-app "install
   * unknown apps" toggle in Settings, so this sends the user there and waits
   * for them to come back via the ActivityCallback below.
   */
  @Command
  fun requestInstallPermission(invoke: Invoke) {
    if (canInstallPackages()) {
      invoke.resolve()
      return
    }
    val intent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES)
    intent.data = Uri.parse("package:${activity.packageName}")
    startActivityForResult(invoke, intent, "onInstallPermissionResult")
  }

  @ActivityCallback
  fun onInstallPermissionResult(invoke: Invoke, @Suppress("UNUSED_PARAMETER") result: Instrumentation.ActivityResult) {
    if (canInstallPackages()) {
      invoke.resolve()
    } else {
      invoke.reject("Install permission was not granted.")
    }
  }

  @Command
  fun install(invoke: Invoke) {
    val args = invoke.parseArgs(InstallArgs::class.java)
    try {
      if (!canInstallPackages()) {
        invoke.reject("Install permission not granted. Call requestInstallPermission first.")
        return
      }
      val file = File(args.path)
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
      val intent = Intent(Intent.ACTION_VIEW)
      intent.setDataAndType(uri, "application/vnd.android.package-archive")
      intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      activity.startActivity(intent)
      invoke.resolve()
    } catch (ex: Exception) {
      invoke.reject(ex.message)
    }
  }

  private fun canInstallPackages(): Boolean {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      activity.packageManager.canRequestPackageInstalls()
    } else {
      // Below Android 8, sideloaded installs are governed by a single global
      // "Unknown sources" toggle, not a per-app grant — nothing to request.
      true
    }
  }
}
