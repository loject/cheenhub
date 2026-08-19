package ru.cheenhub.imagepicker

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia
import java.io.ByteArrayOutputStream

/** Открывает из основной Activity изолированный системный выбор изображения. */
class CheenHubImagePickerPlugin(private val activity: Activity) {
    /** Запускает picker для одного Rust request и сообщает, удалось ли открыть Activity. */
    fun pickImage(requestId: Int): Boolean = runCatching {
        val intent = Intent(activity, CheenHubImagePickerActivity::class.java)
            .putExtra(REQUEST_ID_EXTRA, requestId)
        activity.startActivity(intent)
        Log.d(LOG_TAG, "Opened Android Photo Picker request=$requestId")
    }.onFailure { error ->
        Log.w(LOG_TAG, "Failed to open Android Photo Picker", error)
    }.isSuccess
}

/** Владеет lifecycle системного Photo Picker и чтением выбранного content URI. */
class CheenHubImagePickerActivity : ComponentActivity() {
    private var requestId = INVALID_REQUEST_ID
    private var completed = false

    private val picker = registerForActivityResult(PickVisualMedia()) { uri ->
        if (uri == null) {
            complete(cancelled = true)
        } else {
            readSelectedImage(uri)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestId = savedInstanceState?.getInt(REQUEST_ID_STATE, INVALID_REQUEST_ID)
            ?: intent.getIntExtra(REQUEST_ID_EXTRA, INVALID_REQUEST_ID)
        if (requestId == INVALID_REQUEST_ID) {
            complete(errorCode = "picker_unavailable")
            return
        }
        if (savedInstanceState == null) {
            runCatching {
                picker.launch(PickVisualMediaRequest(PickVisualMedia.ImageOnly))
            }.onFailure { error ->
                Log.w(LOG_TAG, "Android Photo Picker launch failed", error)
                complete(errorCode = "picker_unavailable")
            }
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        outState.putInt(REQUEST_ID_STATE, requestId)
        super.onSaveInstanceState(outState)
    }

    private fun readSelectedImage(uri: Uri) {
        val resolver = applicationContext.contentResolver
        Thread({
            val result = runCatching {
                val contentType = resolver.getType(uri)
                if (contentType != null && !contentType.startsWith("image/")) {
                    throw ImagePickerReadException("unsupported_image")
                }
                val fileName = resolver.query(
                    uri,
                    arrayOf(OpenableColumns.DISPLAY_NAME),
                    null,
                    null,
                    null,
                )?.use { cursor ->
                    val nameColumn = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if (cursor.moveToFirst() && nameColumn >= 0) cursor.getString(nameColumn) else null
                }
                val bytes = resolver.openInputStream(uri)?.use(::readLimitedBytes)
                    ?: throw ImagePickerReadException("read_failed")
                if (bytes.isEmpty()) throw ImagePickerReadException("empty_image")
                fileName to bytes
            }

            result.fold(
                onSuccess = { (fileName, bytes) ->
                    Log.i(LOG_TAG, "Read Android image bytes=${bytes.size}")
                    complete(fileName = fileName, bytes = bytes)
                },
                onFailure = { error ->
                    val errorCode = (error as? ImagePickerReadException)?.code ?: "read_failed"
                    Log.w(LOG_TAG, "Failed to read Android image code=$errorCode")
                    complete(errorCode = errorCode)
                },
            )
        }, "CheenHubImagePicker").start()
    }

    private fun readLimitedBytes(input: java.io.InputStream): ByteArray {
        val output = ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            if (output.size() + read > MAX_IMAGE_BYTES) {
                throw ImagePickerReadException("image_too_large")
            }
            output.write(buffer, 0, read)
        }
        return output.toByteArray()
    }

    @Synchronized
    private fun complete(
        fileName: String? = null,
        bytes: ByteArray? = null,
        errorCode: String? = null,
        cancelled: Boolean = false,
    ) {
        if (completed) return
        completed = true
        nativeOnCheenHubImagePickerResult(
            requestId,
            fileName,
            bytes,
            errorCode,
            cancelled,
        )
        runOnUiThread(::finish)
    }

    private external fun nativeOnCheenHubImagePickerResult(
        requestId: Int,
        fileName: String?,
        bytes: ByteArray?,
        errorCode: String?,
        cancelled: Boolean,
    )
}

private class ImagePickerReadException(val code: String) : Exception(code)

private const val LOG_TAG = "CheenHubImagePicker"
private const val REQUEST_ID_EXTRA = "cheenhub_image_picker_request_id"
private const val REQUEST_ID_STATE = "cheenhub_image_picker_request_id_state"
private const val INVALID_REQUEST_ID = -1
private const val MAX_IMAGE_BYTES = 10 * 1024 * 1024
