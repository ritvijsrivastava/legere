# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# rustls-platform-verifier is called from Rust via JNI, not from any
# visible Java call site, so R8 strips it as unused without this rule --
# causing a ClassNotFoundException crash on every TLS handshake in release
# builds (debug builds aren't minified, so this was invisible locally).
-keep class org.rustls.platformverifier.** { *; }