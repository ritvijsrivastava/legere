buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        // Pinned to the latest 8.x line, not AGP 9.x: AGP 9's default DSL
        // flips to "built-in Kotlin" and drops support for applying the
        // classic org.jetbrains.kotlin.android plugin outright ("The
        // org.jetbrains.kotlin.android plugin is not compatible with the
        // new DSL" - see https://developer.android.com/build/releases/agp-9-0-0-release-notes),
        // which this project's app/build.gradle.kts and buildSrc's custom
        // `rust` plugin (uses com.android.build.api.dsl.ApplicationExtension)
        // both still depend on. Revisit once that migration is done.
        classpath("com.android.tools.build:gradle:8.13.2")
        // Capped below 2.2.x: that's when Kotlin made the old
        // `android { kotlinOptions { jvmTarget = "1.8" } }` DSL (still
        // used by third-party Tauri plugins we don't control - e.g.
        // tauri-android, tauri-plugin-dialog, tauri-plugin-opener's own
        // android/build.gradle.kts in the Cargo registry cache) a hard
        // compile error instead of a deprecation warning.
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.1.21")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

tasks.register("clean").configure {
    delete("build")
}

