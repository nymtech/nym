package net.nymtech.nyms5

import android.util.Log

const val nymNativeLib = "nym_socks5_listener"

class Socks5 {
    private val tag = "Socks5"

    // Load the native library "libnym_socks5_listener.so"
    init {
        System.loadLibrary(nymNativeLib)
        Log.d(tag, "loaded native library $nymNativeLib")
    }

    fun start(serviceProvider: String, callback: Any) {
        Log.d(tag, "calling $nymNativeLib:run")
        try {
            run(serviceProvider, callback)
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:run internal error: $e")
        }
    }

    /* fun start() {
        Log.d(tag, "calling $nymNativeLib:startClient")
        return startClient()
    } */

    fun stop(callback: Any) {
        Log.d(tag, "calling $nymNativeLib:stopClient")
        try {
            stopClient(callback)
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:stopClient internal error: $e")
        }
    }

    private external fun run(spAddress: String, callback: Any)
    private external fun stopClient(callbacks: Any)
    // private external fun startClient()
}