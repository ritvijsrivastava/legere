import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "com.ritvijsrivastava.legere"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.ritvijsrivastava.legere"
        minSdk = 30
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    // Play Store requires every update to this applicationId to be signed
    // with the same key forever. That key is a .jks kept outside the repo;
    // `keystore.properties` (gitignored, see this dir's .gitignore) points at
    // it locally, and CI decodes it from repo secrets — see docs/RELEASING.md.
    // Without this block, `tauri android build --apk` produces an *unsigned*
    // release APK.
    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            }

            keyAlias = keystoreProperties["keyAlias"] as String?
            keyPassword = keystoreProperties["password"] as String?
            storeFile = (keystoreProperties["storeFile"] as String?)?.let { file(it) }
            storePassword = keystoreProperties["password"] as String?
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

// `rustls-platform-verifier` needs a matching Kotlin/JNI component (an
// .aar) at runtime to call into Android's own certificate verifier — the
// Rust crate on its own only gets as far as `ClassNotFoundException:
// org.rustls.platformverifier.CertificateVerifier`. Cargo places this
// alongside `rustls-platform-verifier-android` in its registry cache as a
// tiny local Maven repo; `cargo metadata` is the only reliable way to find
// that path (it varies by registry index hash/crate version). Requires
// `jq` on PATH.
fun findRustlsPlatformVerifierMavenRepo(): File {
    val cargoManifest = File(project.projectDir, "../../../Cargo.toml").canonicalPath
    val manifestPath = providers.exec {
        commandLine(
            "bash", "-c",
            "cargo metadata --format-version 1 --filter-platform x86_64-linux-android " +
                "--manifest-path '$cargoManifest' " +
                "| jq -r '.packages[] | select(.name==\"rustls-platform-verifier-android\") | .manifest_path'"
        )
    }.standardOutput.asText.get().trim()
    return File(File(manifestPath).parentFile, "maven")
}

repositories {
    maven {
        url = uri(findRustlsPlatformVerifierMavenRepo().path)
        metadataSources.artifact()
    }
}

rust {
    // Gradle's rust-build task re-invokes the CLI via `npm run tauri`,
    // which needs a cwd that both (a) has a package.json with a "tauri"
    // script and (b) has `src-tauri` as a *subfolder* (the CLI's own
    // project-discovery only searches downward from its cwd, never
    // sideways or up). Legere's frontend lives in a sibling `frontend/`
    // dir rather than the parent of `src-tauri/` that Tauri's default
    // "../../../" assumes, so neither `frontend/` nor `src-tauri/` alone
    // satisfies both — hence the repo root, plus the delegating
    // `package.json` there (see repo root README/comment).
    rootDirRel = "../../../../"
}

dependencies {
    implementation("rustls:rustls-platform-verifier:latest.release")
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")