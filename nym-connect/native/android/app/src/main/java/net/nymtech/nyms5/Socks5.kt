package net.nymtech.nyms5

import android.util.Log

const val nymNativeLib = "nym_socks5_listener"

class Socks5 {
    // Load the native library "libnym_socks5_listener.so"
    init {
        System.loadLibrary(nymNativeLib)
        Log.d("Socks5", "loaded native library $nymNativeLib")
    }

    fun runClient(): String {
        Log.d("Socks5", "calling $nymNativeLib:run")
        return run("TEST")
    }

    fun start() {
        Log.d("Socks5", "calling $nymNativeLib:startClient")
        return startClient()
    }

    fun stop() {
        Log.d("Socks5", "calling $nymNativeLib:stopClient")
        return stopClient()
    }

    private external fun startClient()
    private external fun stopClient()
    private external fun run(arg: String): String
}