val rExtra = rootProject.extra

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    kotlin("kapt")
    id("com.google.dagger.hilt.android")
    id("org.jetbrains.kotlin.plugin.serialization")
    id("io.objectbox")
}

android {
    namespace = "net.nymtech.nymconnect"
    compileSdk = 34

    val versionMajor = 1
    val versionMinor = 0
    val versionPatch = 0
    val versionBuild = 0

    defaultConfig {
        applicationId = "net.nymtech.nymconnect"
        minSdk = 28
        targetSdk = 34
        versionCode = versionMajor * 10000 + versionMinor * 1000 + versionPatch * 100 + versionBuild
        versionName = "${versionMajor}.${versionMinor}.${versionPatch}"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isDebuggable = false
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    composeOptions {
        kotlinCompilerExtensionVersion = "1.4.8"
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    /* flavorDimensions += "abi"
    productFlavors {
        create("universal") {
            dimension = "abi"
            ndk {
                abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
            }
        }
        create("arch64") {
            dimension = "abi"
            ndk {
                abiFilters += listOf("arm64-v8a", "x86_64")
            }
        }
        create("arm64") {
            dimension = "abi"
            ndk {
                abiFilters += "arm64-v8a"
            }
        }
        create("arm") {
            dimension = "abi"
            ndk {
                abiFilters += "armeabi-v7a"
            }
        }
        create("x86_64") {
            dimension = "abi"
            ndk {
                abiFilters += "x86_64"
            }
        }
        create("x86") {
            dimension = "abi"
            ndk {
                abiFilters += "x86"
            }
        }
    } */
}

dependencies {
    implementation("androidx.core:core-ktx:1.10.1")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.1")
    implementation("androidx.activity:activity-compose:1.7.2")
    implementation(platform("androidx.compose:compose-bom:2023.03.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3:1.1.1")
    implementation("androidx.appcompat:appcompat:1.6.1")

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation(platform("androidx.compose:compose-bom:2023.03.00"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")

    //wireguard tunnel
    implementation("com.wireguard.android:tunnel:1.0.20230706")

    //logging
    implementation("com.jakewharton.timber:timber:5.0.1")

    // compose navigation
    implementation("androidx.navigation:navigation-compose:2.7.1")
    implementation("androidx.hilt:hilt-navigation-compose:1.0.0")

    // hilt
    implementation("com.google.dagger:hilt-android:${rExtra.get("hiltVersion")}")
    kapt("com.google.dagger:hilt-android-compiler:${rExtra.get("hiltVersion")}")

    //accompanist
    implementation("com.google.accompanist:accompanist-systemuicontroller:${rExtra.get("accompanistVersion")}")
    implementation("com.google.accompanist:accompanist-permissions:${rExtra.get("accompanistVersion")}")
    implementation("com.google.accompanist:accompanist-flowlayout:${rExtra.get("accompanistVersion")}")
    implementation("com.google.accompanist:accompanist-navigation-animation:${rExtra.get("accompanistVersion")}")
    implementation("com.google.accompanist:accompanist-drawablepainter:${rExtra.get("accompanistVersion")}")

    //db
    implementation("io.objectbox:objectbox-kotlin:${rExtra.get("objectBoxVersion")}")

    //lifecycle
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.6.1")

    //icons
    implementation("androidx.compose.material:material-icons-extended:1.5.0")


    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.5.1")
}

kapt {
    correctErrorTypes = true
}