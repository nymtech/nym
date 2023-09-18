package net.nymtech.nymconnect.service.tunnel.model

import com.wireguard.config.Config
import io.objectbox.annotation.ConflictStrategy
import io.objectbox.annotation.Entity
import io.objectbox.annotation.Id
import io.objectbox.annotation.Unique
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.InputStream


@Entity
@Serializable
data class TunnelConfig(
    @Id
    var id : Long = 0,
    @Unique(onConflict = ConflictStrategy.REPLACE)
    var name : String,
    var wgQuick : String
) {


    override fun toString(): String {
        return Json.encodeToString(serializer(), this)
    }

    companion object {
        private const val INCLUDED_APPLICATIONS = "IncludedApplications = "
        private const val EXCLUDED_APPLICATIONS = "ExcludedApplications = "
        private const val INTERFACE = "[Interface]"
        private const val NEWLINE_CHAR = "\n"
        private const val APP_CONFIG_SEPARATOR = ", "

        private fun addApplicationsToConfig(appConfig : String, wgQuick : String) : String {
            val configList = wgQuick.split(NEWLINE_CHAR).toMutableList()
            val interfaceIndex = configList.indexOf(INTERFACE)
            configList.add(interfaceIndex + 1, appConfig)
            return configList.joinToString(NEWLINE_CHAR)
        }

        fun clearAllApplicationsFromConfig(wgQuick : String) : String {
            val configList = wgQuick.split(NEWLINE_CHAR).toMutableList()
            val itr = configList.iterator()
            while (itr.hasNext()) {
                val next = itr.next()
                if(next.contains(INCLUDED_APPLICATIONS) || next.contains(EXCLUDED_APPLICATIONS)) {
                    itr.remove()
                }
            }
            return configList.joinToString(NEWLINE_CHAR)
        }


        fun setExcludedApplicationsOnQuick(packages : List<String>, wgQuick: String) : String {
            if(packages.isEmpty()) {
                return wgQuick
            }
            val clearedWgQuick = clearAllApplicationsFromConfig(wgQuick)
            val excludeConfig = buildExcludedApplicationsString(packages)
            return addApplicationsToConfig(excludeConfig, clearedWgQuick)
        }

        fun setIncludedApplicationsOnQuick(packages : List<String>, wgQuick: String) : String {
            if(packages.isEmpty()) {
                return wgQuick
            }
            val clearedWgQuick = clearAllApplicationsFromConfig(wgQuick)
            val includeConfig = buildIncludedApplicationsString(packages)
            return addApplicationsToConfig(includeConfig, clearedWgQuick)
        }

        private fun buildExcludedApplicationsString(packages : List<String>) : String {
            return EXCLUDED_APPLICATIONS + packages.joinToString(APP_CONFIG_SEPARATOR)
        }

        private fun buildIncludedApplicationsString(packages : List<String>) : String {
            return INCLUDED_APPLICATIONS + packages.joinToString(APP_CONFIG_SEPARATOR)
        }
        fun from(string : String) : TunnelConfig {
            return Json.decodeFromString<TunnelConfig>(string)
        }
        fun configFromQuick(wgQuick: String): Config {
            val inputStream: InputStream = wgQuick.byteInputStream()
            val reader = inputStream.bufferedReader(Charsets.UTF_8)
            return Config.parse(reader)
        }
    }
}