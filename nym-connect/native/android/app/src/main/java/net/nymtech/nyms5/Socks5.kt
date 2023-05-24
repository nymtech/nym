package net.nymtech.nyms5

import android.util.Log

const val nymNativeLib = "nym_socks5_listener"

class Socks5 {
    private val tag = "Socks5"

    companion object Cb {
        private const val tag = "Socks5:Cb"

        fun onStart() {
            Log.w(tag, "⚡⚡⚡⚡ CB START ⚡⚡⚡⚡")
        }

        fun onStop() {
            Log.w(tag, "⚡⚡⚡⚡ CB STOP ⚡⚡⚡⚡")
        }
    }

    // Load the native library "libnym_socks5_listener.so"
    init {
        System.loadLibrary(nymNativeLib)
        Log.d(tag, "loaded native library $nymNativeLib")
    }

    fun start(serviceProvider: String) {
        Log.d(tag, "calling $nymNativeLib:run")
        try {
            run(serviceProvider, Cb)
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:run internal error: $e")
        }
    }

    /* fun start() {
        Log.d(tag, "calling $nymNativeLib:startClient")
        return startClient()
    } */

    fun stop() {
        Log.d(tag, "calling $nymNativeLib:stopClient")
        try {
            stopClient(Cb)
        } catch (e: Throwable) {
            Log.e(tag, "$nymNativeLib:stopClient internal error: $e")
        }
    }

    private external fun run(spAddress: String, callbacks: Cb)
    private external fun stopClient(callbacks: Cb)
    private external fun startClient()
}