plugins {
  id("com.android.application")
  id("org.jetbrains.kotlin.android")
  id("rustPlugin")
}

android {
  compileSdk = 33
  defaultConfig {
    manifestPlaceholders["usesCleartextTraffic"] = "false"
    applicationId = "net.nymtech.nym_connect_android"
    minSdk = 24
    targetSdk = 33
    versionCode = 1
    versionName = "1.0"
  }
  sourceSets.getByName("main") {
    // Vulkan validation layers
    val ndkHome = System.getenv("NDK_HOME")
    jniLibs.srcDir("${ndkHome}/sources/third_party/vulkan/src/build-android/jniLibs")
  }
  buildTypes {
    getByName("debug") {
      manifestPlaceholders["usesCleartextTraffic"] = "true"
      isDebuggable = true
      isJniDebuggable = true
      isMinifyEnabled = false
      packagingOptions {
        jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")

        jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")

        jniLibs.keepDebugSymbols.add("*/x86/*.so")

        jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
      }
    }
    getByName("release") {
      isMinifyEnabled = false
    //  proguardFiles(getDefaultProguardFile("proguard-android.txt"), "proguard-rules.pro")
    }
  }
  flavorDimensions.add("abi")
  productFlavors {
    create("universal") {
      val abiList = findProperty("abiList") as? String

      dimension = "abi"
      ndk {
        abiFilters += abiList?.split(",")?.map { it.trim() } ?: listOf(
          "arm64-v8a", "armeabi-v7a", "x86", "x86_64",
        )
      }
    }
    create("arm64") {
      dimension = "abi"
      ndk {
        abiFilters += listOf("arm64-v8a")
      }
    }

    create("arm") {
      dimension = "abi"
      ndk {
        abiFilters += listOf("armeabi-v7a")
      }
    }

    create("x86") {
      dimension = "abi"
      ndk {
        abiFilters += listOf("x86")
      }
    }

    create("x86_64") {
      dimension = "abi"
      ndk {
        abiFilters += listOf("x86_64")
      }
    }
  }

  assetPacks += mutableSetOf()
  namespace = "net.nymtech.nym_connect_android"
}

rust {
  rootDirRel = "../../../../"
  targets = listOf("aarch64", "armv7", "i686", "x86_64")
  arches = listOf("arm64", "arm", "x86", "x86_64")
}

dependencies {
  implementation("androidx.webkit:webkit:1.5.0")
  implementation("androidx.appcompat:appcompat:1.5.1")
  implementation("com.google.android.material:material:1.7.0")
  testImplementation("junit:junit:4.13.2")
  androidTestImplementation("androidx.test.ext:junit:1.1.4")
  androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

afterEvaluate {
  android.applicationVariants.all {
    tasks["mergeUniversalReleaseJniLibFolders"].dependsOn(tasks["rustBuildRelease"])
    tasks["mergeUniversalDebugJniLibFolders"].dependsOn(tasks["rustBuildDebug"])
    productFlavors.filter { it.name != "universal" }.forEach { _ ->
      val archAndBuildType = name.capitalize()
      tasks["merge${archAndBuildType}JniLibFolders"].dependsOn(tasks["rustBuild${archAndBuildType}"])
    }
  }
}
