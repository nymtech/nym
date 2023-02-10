package net.nymtech

import java.io.File
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.logging.LogLevel
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction

open class BuildTask : DefaultTask() {
    @InputDirectory
    @PathSensitive(PathSensitivity.RELATIVE)
    var rootDirRel: File? = null

    @Input
    var target: String? = null

    @Input
    var release: Boolean? = null

    @TaskAction
    fun build() {
        val rootDirRel = rootDirRel ?: throw GradleException("rootDirRel cannot be null")
        val target = target ?: throw GradleException("target cannot be null")
        val release = release ?: throw GradleException("release cannot be null")
        val home = (System.getenv("HOME") ?: "")
        val cargoHome = (System.getenv("CARGO_HOME") ?: "$home/.cargo")
        val tauriCli = "$cargoHome/bin/cargo-tauri"
        if (!File(tauriCli).isFile()) {
            throw GradleException("$tauriCli no shuch file")
        }
        println("gradle Rust plugin, using tauri cli executable: $tauriCli")
        project.exec {
            workingDir(File(project.projectDir, rootDirRel.path))
            executable(tauriCli)
            args(listOf("tauri", "android", "android-studio-script"))
            if (project.logger.isEnabled(LogLevel.DEBUG)) {
                args("-vv")
            } else if (project.logger.isEnabled(LogLevel.INFO)) {
                args("-v")
            }
            if (release) {
                args("--release")
            }
            args(listOf("--target", target))
        }.assertNormalExitValue()
    }
}