package com.gym_manager.dc

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.util.Base64
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class OpenPdfArgs {
    var base64: String? = null
    var path: String? = null
}

@TauriPlugin
class PdfPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun openPdf(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(OpenPdfArgs::class.java)
            val base64 = args.base64
            val path = args.path

            val file = when {
                !base64.isNullOrBlank() -> {
                    // Write the PDF directly into the app's internal cache.
                    // FileProvider's <cache-path> covers this directory, so it
                    // does not depend on Tauri's path resolver/FileProvider roots.
                    val bytes = Base64.decode(base64, Base64.DEFAULT)
                    val pdf = File(activity.cacheDir, "barcode-${System.currentTimeMillis()}.pdf")
                    pdf.writeBytes(bytes)
                    pdf
                }

                !path.isNullOrBlank() -> File(path)

                else -> {
                    invoke.reject("PDF no proporcionado")
                    return
                }
            }

            if (!file.exists() || file.length() == 0L) {
                invoke.reject("PDF no encontrado o vacío")
                return
            }

            val uri: Uri = FileProvider.getUriForFile(
                activity,
                "${activity.packageName}.fileprovider",
                file
            )

            val viewIntent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/pdf")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }

            try {
                activity.startActivity(viewIntent)
            } catch (_: Exception) {
                // Si no hay visor de PDF, ofrecer compartir el archivo.
                val shareIntent = Intent(Intent.ACTION_SEND).apply {
                    type = "application/pdf"
                    putExtra(Intent.EXTRA_STREAM, uri)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                activity.startActivity(Intent.createChooser(shareIntent, "Abrir PDF"))
            }

            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject(e.message)
        }
    }

    @Command
    fun shareFile(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(OpenPdfArgs::class.java)
            val path = args.path
            if (path.isNullOrBlank()) {
                invoke.reject("Archivo no proporcionado")
                return
            }

            val file = File(path)
            if (!file.exists() || file.length() == 0L) {
                invoke.reject("Archivo no encontrado o vacío")
                return
            }

            val uri: Uri = FileProvider.getUriForFile(
                activity,
                "${activity.packageName}.fileprovider",
                file
            )

            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "application/zip"
                putExtra(Intent.EXTRA_STREAM, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }

            activity.startActivity(Intent.createChooser(shareIntent, "Guardar o compartir backup"))
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            invoke.reject(e.message)
        }
    }
}
