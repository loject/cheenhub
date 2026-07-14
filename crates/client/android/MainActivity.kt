package dev.dioxus.main

import android.app.Activity
import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.util.Log
import android.util.Range
import android.view.Surface
import com.google.firebase.FirebaseApp
import com.google.firebase.FirebaseOptions
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import org.json.JSONArray
import org.json.JSONObject
import java.text.ParseException
import java.text.SimpleDateFormat
import java.util.Locale
import java.util.TimeZone
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

typealias BuildConfig = ru.cheenhub.BuildConfig

class MainActivity : WryActivity() {
    private val captures = ConcurrentHashMap<Int, CaptureResources>()
    private var voiceAudioFocus: AudioFocusRequest? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        CheenHubPushStore.setAppForeground(this, true)
        acceptNotificationIntent(intent)
    }

    override fun onStart() {
        super.onStart()
        CheenHubPushStore.setAppForeground(this, true)
    }

    override fun onStop() {
        CheenHubPushStore.setAppForeground(this, false)
        super.onStop()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        acceptNotificationIntent(intent)
    }

    fun requestCheenHubNotificationPermission(requestCode: Int) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), requestCode)
        } else {
            nativeOnCheenHubPermissionResult(requestCode, true, false)
        }
    }

    fun requestCheenHubPushInstallation(requestId: Int) {
        val installationId = CheenHubPushStore.installationId(this)
        if (!CheenHubFirebase.initialize(this)) {
            nativeOnCheenHubPushInstallationResult(
                requestId,
                installationId,
                null,
                "firebase_not_configured",
            )
            return
        }
        FirebaseMessaging.getInstance().token.addOnCompleteListener { task ->
            val token = if (task.isSuccessful) task.result?.takeIf(String::isNotBlank) else null
            if (token != null) {
                CheenHubPushStore.saveFcmToken(this, token)
                Log.i(CHEENHUB_PUSH_LOG_TAG, "FCM installation is ready for synchronization")
            } else {
                Log.w(CHEENHUB_PUSH_LOG_TAG, "FCM token request failed")
            }
            nativeOnCheenHubPushInstallationResult(
                requestId,
                installationId,
                token,
                if (token == null) "fcm_token_unavailable" else null,
            )
        }
    }

    fun consumeCheenHubPendingDirectMessageConversationId(): String? =
        CheenHubPushStore.consumePendingConversationId(this)

    fun setCheenHubActiveDirectMessageConversationId(conversationId: String?) {
        CheenHubPushStore.setActiveConversationId(this, conversationId)
    }

    fun clearCheenHubDirectMessageNotification(conversationId: String) {
        CheenHubPushStore.clearConversation(this, conversationId)
    }

    private fun acceptNotificationIntent(intent: Intent?) {
        if (intent?.action != CHEENHUB_OPEN_DIRECT_MESSAGE_ACTION) return
        val conversationId = intent.getStringExtra(CHEENHUB_CONVERSATION_ID_EXTRA) ?: return
        CheenHubPushStore.setPendingConversationId(this, conversationId)
        runCatching { nativeOnCheenHubDirectMessageNotificationOpened(conversationId) }
            .onFailure {
                Log.d(
                    CHEENHUB_PUSH_LOG_TAG,
                    "Native notification-open callback is not ready; pending destination was stored",
                )
            }
        Log.i(CHEENHUB_PUSH_LOG_TAG, "Direct-message notification intent accepted")
    }

    fun requestCheenHubPermission(permission: String, requestCode: Int) {
        requestPermissions(arrayOf(permission), requestCode)
    }

    fun requestCheenHubMediaProjection(requestCode: Int) {
        val manager = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        startActivityForResult(manager.createScreenCaptureIntent(), requestCode)
    }

    fun startCheenHubForegroundService(kind: String) {
        if (kind == "voice") configureVoiceAudio()
        val intent = Intent(this, DioxusForegroundService::class.java)
            .setAction(DioxusForegroundService.ACTION_START)
            .putExtra(DioxusForegroundService.EXTRA_KIND, kind)
        startForegroundService(intent)
    }

    fun stopCheenHubForegroundService(kind: String) {
        if (kind == "voice") releaseVoiceAudio()
        val intent = Intent(this, DioxusForegroundService::class.java)
            .setAction(DioxusForegroundService.ACTION_STOP)
            .putExtra(DioxusForegroundService.EXTRA_KIND, kind)
        startService(intent)
    }

    fun startCheenHubCamera(
        captureId: Int,
        surface: Surface,
        width: Int,
        height: Int,
        frameRate: Int,
    ) {
        require(width > 0 && height > 0 && frameRate > 0) { "Invalid camera capture configuration" }
        check(!captures.containsKey(captureId)) { "Capture id is already active" }

        val cameraManager = getSystemService(CameraManager::class.java)
        val cameraId = selectCamera(cameraManager)
        val thread = HandlerThread("cheenhub-camera-$captureId").apply { start() }
        val handler = Handler(thread.looper)
        val resources = CameraResources(thread, surface)
        captures[captureId] = resources
        val started = CountDownLatch(1)

        try {
            cameraManager.openCamera(cameraId, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    resources.camera = camera
                    val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_RECORD).apply {
                        addTarget(surface)
                        set(CaptureRequest.CONTROL_MODE, CaptureRequest.CONTROL_MODE_AUTO)
                        selectFpsRange(cameraManager, cameraId, frameRate)?.let {
                            set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, it)
                        }
                    }
                    camera.createCaptureSession(listOf(surface), object : CameraCaptureSession.StateCallback() {
                        override fun onConfigured(session: CameraCaptureSession) {
                            resources.session = session
                            runCatching { session.setRepeatingRequest(request.build(), null, handler) }
                                .onFailure { resources.startError = it }
                            started.countDown()
                        }

                        override fun onConfigureFailed(session: CameraCaptureSession) {
                            resources.startError = IllegalStateException("Camera2 capture session configuration failed")
                            started.countDown()
                        }
                    }, handler)
                }

                override fun onDisconnected(camera: CameraDevice) {
                    camera.close()
                    resources.startError = IllegalStateException("Camera was disconnected")
                    started.countDown()
                    captureEnded(captureId)
                }

                override fun onError(camera: CameraDevice, error: Int) {
                    camera.close()
                    resources.startError = IllegalStateException("Camera2 error: $error")
                    started.countDown()
                    captureEnded(captureId)
                }
            }, handler)
        } catch (error: Throwable) {
            captures.remove(captureId)
            resources.close()
            throw error
        }

        if (!started.await(CAPTURE_START_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
            captures.remove(captureId)
            resources.close()
            throw IllegalStateException("Timed out while starting Camera2 capture")
        }
        resources.startError?.let { error ->
            captures.remove(captureId)
            resources.close()
            throw error
        }
    }

    fun startCheenHubScreenShare(
        captureId: Int,
        grant: Intent,
        surface: Surface,
        width: Int,
        height: Int,
    ) {
        require(width > 0 && height > 0) { "Invalid screen capture dimensions" }
        check(!captures.containsKey(captureId)) { "Capture id is already active" }
        val manager = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        val projection = manager.getMediaProjection(Activity.RESULT_OK, grant)
            ?: throw IllegalStateException("MediaProjection grant is no longer valid")
        val resources = ProjectionResources(projection, surface)
        captures[captureId] = resources
        projection.registerCallback(object : MediaProjection.Callback() {
            override fun onStop() = captureEnded(captureId)
        }, Handler(mainLooper))
        try {
            resources.display = projection.createVirtualDisplay(
                "CheenHub screen share",
                width,
                height,
                resources.densityDpi,
                android.hardware.display.DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                surface,
                null,
                null,
            )
        } catch (error: Throwable) {
            captures.remove(captureId)
            resources.close()
            throw error
        }
    }

    fun stopCheenHubCapture(captureId: Int) {
        captures.remove(captureId)?.close()
    }

    private fun captureEnded(captureId: Int) {
        val resources = captures.remove(captureId) ?: return
        resources.close()
        nativeOnCheenHubCaptureEnded(captureId)
    }

    private fun selectCamera(manager: CameraManager): String {
        return manager.cameraIdList.firstOrNull { id ->
            manager.getCameraCharacteristics(id).get(CameraCharacteristics.LENS_FACING) ==
                CameraCharacteristics.LENS_FACING_FRONT
        } ?: manager.cameraIdList.firstOrNull()
        ?: throw IllegalStateException("No Android camera is available")
    }

    private fun configureVoiceAudio() {
        val manager = getSystemService(AudioManager::class.java)
        manager.mode = AudioManager.MODE_IN_COMMUNICATION
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                        .build(),
                )
                .setOnAudioFocusChangeListener { }
                .build()
            voiceAudioFocus = request
            manager.requestAudioFocus(request)
        } else {
            @Suppress("DEPRECATION")
            manager.requestAudioFocus(
                null,
                AudioManager.STREAM_VOICE_CALL,
                AudioManager.AUDIOFOCUS_GAIN,
            )
        }
    }

    private fun releaseVoiceAudio() {
        val manager = getSystemService(AudioManager::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            voiceAudioFocus?.let(manager::abandonAudioFocusRequest)
            voiceAudioFocus = null
        } else {
            @Suppress("DEPRECATION")
            manager.abandonAudioFocus(null)
        }
        manager.mode = AudioManager.MODE_NORMAL
    }

    private fun selectFpsRange(manager: CameraManager, cameraId: String, requested: Int): Range<Int>? {
        val ranges = manager.getCameraCharacteristics(cameraId)
            .get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
            ?: return null
        return ranges.filter { requested in it }.minByOrNull { it.upper - it.lower }
            ?: ranges.minByOrNull { kotlin.math.abs(it.upper - requested) }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        val granted = grantResults.firstOrNull() == android.content.pm.PackageManager.PERMISSION_GRANTED
        val canAskAgain = permissions.firstOrNull()?.let(::shouldShowRequestPermissionRationale) ?: false
        nativeOnCheenHubPermissionResult(requestCode, granted, canAskAgain)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        nativeOnCheenHubMediaProjectionResult(requestCode, resultCode == Activity.RESULT_OK, data)
    }

    private external fun nativeOnCheenHubPermissionResult(
        requestCode: Int,
        granted: Boolean,
        canAskAgain: Boolean,
    )

    private external fun nativeOnCheenHubMediaProjectionResult(
        requestCode: Int,
        granted: Boolean,
        data: Intent?,
    )

    private external fun nativeOnCheenHubCaptureEnded(captureId: Int)

    private external fun nativeOnCheenHubPushInstallationResult(
        requestId: Int,
        installationId: String,
        token: String?,
        errorCode: String?,
    )

    private external fun nativeOnCheenHubDirectMessageNotificationOpened(conversationId: String)

    override fun onDestroy() {
        captures.values.forEach(CaptureResources::close)
        captures.clear()
        releaseVoiceAudio()
        super.onDestroy()
    }

    private sealed interface CaptureResources {
        fun close()
    }

    private class CameraResources(
        private val thread: HandlerThread,
        private val surface: Surface,
    ) : CaptureResources {
        @Volatile var camera: CameraDevice? = null
        @Volatile var session: CameraCaptureSession? = null
        @Volatile var startError: Throwable? = null

        override fun close() {
            runCatching { session?.stopRepeating() }
            session?.close()
            camera?.close()
            surface.release()
            thread.quitSafely()
        }
    }

    private inner class ProjectionResources(
        private val projection: MediaProjection,
        private val surface: Surface,
    ) : CaptureResources {
        var display: android.hardware.display.VirtualDisplay? = null
        val densityDpi: Int get() = resources.configuration.densityDpi

        override fun close() {
            display?.release()
            projection.stop()
            surface.release()
        }
    }

    companion object {
        private const val CAPTURE_START_TIMEOUT_SECONDS = 5L
    }
}

private const val CHEENHUB_PUSH_LOG_TAG = "CheenHubPush"
private const val CHEENHUB_OPEN_DIRECT_MESSAGE_ACTION =
    "ru.cheenhub.action.OPEN_DIRECT_MESSAGE"
private const val CHEENHUB_CONVERSATION_ID_EXTRA = "cheenhub_conversation_id"

class CheenHubApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        CheenHubPushStore.setAppForeground(this, false)
        CheenHubNotifications.ensureChannel(this)
        CheenHubFirebase.initialize(this)
    }
}

object CheenHubFirebase {
    private const val META_PROJECT_ID = "ru.cheenhub.firebase.project_id"
    private const val META_APPLICATION_ID = "ru.cheenhub.firebase.application_id"
    private const val META_API_KEY = "ru.cheenhub.firebase.api_key"
    private const val META_SENDER_ID = "ru.cheenhub.firebase.sender_id"
    private const val UNCONFIGURED_PREFIX = "CONFIGURE_CHEENHUB_"

    @Synchronized
    fun initialize(context: Context): Boolean {
        if (FirebaseApp.getApps(context).isNotEmpty()) return true
        val metadata = runCatching {
            context.packageManager.getApplicationInfo(
                context.packageName,
                android.content.pm.PackageManager.GET_META_DATA,
            ).metaData
        }.getOrElse { error ->
            Log.e(CHEENHUB_PUSH_LOG_TAG, "Firebase metadata lookup failed", error)
            return false
        }
        val projectId = metadata?.getString(META_PROJECT_ID).configuredValue()
        val applicationId = metadata?.getString(META_APPLICATION_ID).configuredValue()
        val apiKey = metadata?.getString(META_API_KEY).configuredValue()
        val senderId = metadata?.getString(META_SENDER_ID).configuredValue()
        if (projectId == null || applicationId == null || apiKey == null || senderId == null) {
            Log.w(
                CHEENHUB_PUSH_LOG_TAG,
                "FCM is disabled: configure Firebase project_id, application_id, api_key and sender_id metadata",
            )
            return false
        }
        val options = FirebaseOptions.Builder()
            .setProjectId(projectId)
            .setApplicationId(applicationId)
            .setApiKey(apiKey)
            .setGcmSenderId(senderId)
            .build()
        return runCatching {
            FirebaseApp.initializeApp(context, options)
            true
        }
            .onSuccess { Log.i(CHEENHUB_PUSH_LOG_TAG, "Firebase application initialized") }
            .onFailure { error ->
                Log.e(CHEENHUB_PUSH_LOG_TAG, "Firebase application initialization failed", error)
            }
            .getOrDefault(false)
    }

    private fun String?.configuredValue(): String? = this
        ?.trim()
        ?.takeIf { it.isNotEmpty() && !it.startsWith(UNCONFIGURED_PREFIX) }
}

class CheenHubFirebaseMessagingService : FirebaseMessagingService() {
    override fun onNewToken(token: String) {
        super.onNewToken(token)
        CheenHubPushStore.saveFcmToken(this, token)
        Log.i(CHEENHUB_PUSH_LOG_TAG, "FCM token was refreshed and stored")
    }

    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        super.onMessageReceived(remoteMessage)
        val message = CheenHubDirectMessagePayload.parse(remoteMessage.data)
        if (message == null) {
            Log.w(CHEENHUB_PUSH_LOG_TAG, "Rejected malformed or unsupported FCM data payload")
            return
        }
        val history = CheenHubPushStore.appendMessage(this, message)
        if (history == null) {
            Log.d(
                CHEENHUB_PUSH_LOG_TAG,
                "Ignored duplicate direct-message push",
            )
            return
        }
        if (CheenHubPushStore.shouldSuppress(this, message.conversationId)) {
            CheenHubPushStore.clearConversation(this, message.conversationId)
            Log.d(
                CHEENHUB_PUSH_LOG_TAG,
                "Suppressed direct-message notification for active conversation",
            )
            return
        }
        CheenHubNotifications.showConversation(this, history)
        Log.i(
            CHEENHUB_PUSH_LOG_TAG,
            "Direct-message notification updated; messages=${history.messages.size}",
        )
    }
}

private data class CheenHubDirectMessagePayload(
    val messageId: String,
    val conversationId: String,
    val sequence: Long,
    val senderUserId: String,
    val senderNickname: String,
    val bodyPreview: String,
    val createdAtMillis: Long,
) {
    companion object {
        private const val SCHEMA_VERSION = "1"
        private const val KIND = "direct_message"
        private const val MAX_NICKNAME_LENGTH = 100
        private const val MAX_BODY_LENGTH = 500

        fun parse(data: Map<String, String>): CheenHubDirectMessagePayload? {
            if (data["schema_version"] != SCHEMA_VERSION || data["kind"] != KIND) return null
            val messageId = data["message_id"].validUuid() ?: return null
            val conversationId = data["conversation_id"].validUuid() ?: return null
            val sequence = data["message_seq"]?.toLongOrNull()?.takeIf { it > 0 } ?: return null
            val senderUserId = data["sender_user_id"].validUuid() ?: return null
            val nickname = data["sender_nickname"].boundedText(MAX_NICKNAME_LENGTH) ?: return null
            val body = data["body_preview"].boundedText(MAX_BODY_LENGTH) ?: return null
            val createdAt = parseTimestamp(data["created_at"] ?: return null) ?: return null
            return CheenHubDirectMessagePayload(
                messageId,
                conversationId,
                sequence,
                senderUserId,
                nickname,
                body,
                createdAt,
            )
        }

        private fun String?.validUuid(): String? = this?.let { value ->
            runCatching { UUID.fromString(value).toString() }.getOrNull()
        }

        private fun String?.boundedText(maxLength: Int): String? = this
            ?.trim()
            ?.takeIf {
                it.isNotEmpty() && it.codePointCount(0, it.length) <= maxLength
            }

        private fun parseTimestamp(value: String): Long? {
            value.toLongOrNull()?.takeIf { it > 0 }?.let { return it }
            val patterns = listOf(
                "yyyy-MM-dd'T'HH:mm:ss.SSSSSSSSSXXX",
                "yyyy-MM-dd'T'HH:mm:ss.SSSXXX",
                "yyyy-MM-dd'T'HH:mm:ssXXX",
            )
            for (pattern in patterns) {
                try {
                    return SimpleDateFormat(pattern, Locale.US).apply {
                        isLenient = false
                        timeZone = TimeZone.getTimeZone("UTC")
                    }.parse(value)?.time
                } catch (_: ParseException) {
                    // Следующий формат проверяется без вывода содержимого payload в лог.
                }
            }
            return null
        }
    }
}

private data class CheenHubConversationHistory(
    val conversationId: String,
    val senderUserId: String,
    val senderNickname: String,
    val messages: List<CheenHubStoredMessage>,
)

private data class CheenHubStoredMessage(
    val messageId: String,
    val sequence: Long,
    val bodyPreview: String,
    val createdAtMillis: Long,
)

private object CheenHubPushStore {
    private const val PREFERENCES = "cheenhub_push"
    private const val INSTALLATION_ID = "installation_id"
    private const val FCM_TOKEN = "fcm_token"
    private const val PENDING_CONVERSATION = "pending_conversation_id"
    private const val ACTIVE_CONVERSATION = "active_conversation_id"
    private const val APP_FOREGROUND = "app_foreground"
    private const val HISTORY = "direct_message_history"
    private const val MAX_CONVERSATIONS = 20
    private const val MAX_MESSAGES_PER_CONVERSATION = 10
    private const val MAX_SEEN_MESSAGE_IDS = 200

    fun installationId(context: Context): String = synchronized(this) {
        val preferences = preferences(context)
        preferences.getString(INSTALLATION_ID, null) ?: UUID.randomUUID().toString().also {
            preferences.edit().putString(INSTALLATION_ID, it).apply()
        }
    }

    fun saveFcmToken(context: Context, token: String) {
        if (token.isBlank()) return
        preferences(context).edit().putString(FCM_TOKEN, token).apply()
    }

    fun setPendingConversationId(context: Context, conversationId: String) {
        val normalized = conversationId.validUuidOrNull() ?: return
        preferences(context).edit().putString(PENDING_CONVERSATION, normalized).apply()
    }

    fun consumePendingConversationId(context: Context): String? = synchronized(this) {
        val preferences = preferences(context)
        preferences.getString(PENDING_CONVERSATION, null)?.also {
            preferences.edit().remove(PENDING_CONVERSATION).apply()
        }
    }

    fun setActiveConversationId(context: Context, conversationId: String?) {
        val editor = preferences(context).edit()
        val normalized = conversationId.validUuidOrNull()
        if (normalized == null) editor.remove(ACTIVE_CONVERSATION)
        else editor.putString(ACTIVE_CONVERSATION, normalized)
        editor.apply()
    }

    fun setAppForeground(context: Context, foreground: Boolean) {
        preferences(context).edit().putBoolean(APP_FOREGROUND, foreground).apply()
    }

    fun shouldSuppress(context: Context, conversationId: String): Boolean {
        val preferences = preferences(context)
        return preferences.getBoolean(APP_FOREGROUND, false) &&
            preferences.getString(ACTIVE_CONVERSATION, null) == conversationId
    }

    fun appendMessage(
        context: Context,
        payload: CheenHubDirectMessagePayload,
    ): CheenHubConversationHistory? = synchronized(this) {
        val root = readRoot(context)
        val seen = root.optJSONArray("seen_message_ids") ?: JSONArray()
        if ((0 until seen.length()).any { seen.optString(it) == payload.messageId }) return null
        seen.put(payload.messageId)
        while (seen.length() > MAX_SEEN_MESSAGE_IDS) removeFirst(seen)
        root.put("seen_message_ids", seen)

        val conversations = root.optJSONObject("conversations") ?: JSONObject()
        val conversation = conversations.optJSONObject(payload.conversationId) ?: JSONObject()
        val messages = conversation.optJSONArray("messages") ?: JSONArray()
        messages.put(
            JSONObject()
                .put("message_id", payload.messageId)
                .put("sequence", payload.sequence)
                .put("body_preview", payload.bodyPreview)
                .put("created_at", payload.createdAtMillis),
        )
        val sortedMessages = (0 until messages.length())
            .mapNotNull(messages::optJSONObject)
            .sortedWith(compareBy({ it.optLong("sequence") }, { it.optLong("created_at") }))
            .takeLast(MAX_MESSAGES_PER_CONVERSATION)
        val boundedMessages = JSONArray().apply { sortedMessages.forEach(::put) }
        conversation
            .put("sender_user_id", payload.senderUserId)
            .put("sender_nickname", payload.senderNickname)
            .put("updated_at", payload.createdAtMillis)
            .put("messages", boundedMessages)
        conversations.put(payload.conversationId, conversation)
        trimConversations(conversations, context)
        root.put("conversations", conversations)
        writeRoot(context, root)
        conversationHistory(payload.conversationId, conversation)
    }

    fun clearConversation(context: Context, conversationId: String) = synchronized(this) {
        val normalized = conversationId.validUuidOrNull() ?: return
        val root = readRoot(context)
        root.optJSONObject("conversations")?.remove(normalized)
        writeRoot(context, root)
        context.getSystemService(NotificationManager::class.java)
            .cancel(notificationTag(normalized), CheenHubNotifications.CONVERSATION_NOTIFICATION_ID)
    }

    private fun trimConversations(conversations: JSONObject, context: Context) {
        val ids = conversations.keys().asSequence().toList()
        if (ids.size <= MAX_CONVERSATIONS) return
        ids.sortedBy { conversations.optJSONObject(it)?.optLong("updated_at") ?: 0L }
            .take(ids.size - MAX_CONVERSATIONS)
            .forEach { id ->
                conversations.remove(id)
                context.getSystemService(NotificationManager::class.java)
                    .cancel(notificationTag(id), CheenHubNotifications.CONVERSATION_NOTIFICATION_ID)
            }
    }

    private fun conversationHistory(
        conversationId: String,
        conversation: JSONObject,
    ): CheenHubConversationHistory {
        val messages = conversation.optJSONArray("messages") ?: JSONArray()
        return CheenHubConversationHistory(
            conversationId,
            conversation.optString("sender_user_id"),
            conversation.optString("sender_nickname"),
            (0 until messages.length()).mapNotNull { index ->
                messages.optJSONObject(index)?.let { message ->
                    CheenHubStoredMessage(
                        message.optString("message_id"),
                        message.optLong("sequence"),
                        message.optString("body_preview"),
                        message.optLong("created_at"),
                    )
                }
            },
        )
    }

    private fun readRoot(context: Context): JSONObject {
        val stored = preferences(context).getString(HISTORY, null) ?: return JSONObject()
        return runCatching { JSONObject(stored) }.getOrElse {
            Log.w(CHEENHUB_PUSH_LOG_TAG, "Discarded invalid local notification history")
            JSONObject()
        }
    }

    private fun writeRoot(context: Context, root: JSONObject) {
        preferences(context).edit().putString(HISTORY, root.toString()).apply()
    }

    private fun removeFirst(array: JSONArray) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.KITKAT) array.remove(0)
    }

    private fun preferences(context: Context) =
        context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)

    private fun String?.validUuidOrNull(): String? = this?.let {
        runCatching { UUID.fromString(it).toString() }.getOrNull()
    }

    private fun notificationTag(conversationId: String) = "cheenhub_dm:$conversationId"
}

private object CheenHubNotifications {
    const val CONVERSATION_NOTIFICATION_ID = 2001
    private const val CHANNEL_ID = "cheenhub_direct_messages"

    fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Личные сообщения",
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = "Уведомления о новых личных сообщениях CheenHub"
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    fun showConversation(context: Context, history: CheenHubConversationHistory) {
        ensureChannel(context)
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            android.app.Notification.Builder(context, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            android.app.Notification.Builder(context)
        }
        val style = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val currentUser = android.app.Person.Builder()
                .setKey("cheenhub_current_user")
                .setName("Вы")
                .build()
            val sender = android.app.Person.Builder()
                .setKey(history.senderUserId)
                .setName(history.senderNickname)
                .build()
            android.app.Notification.MessagingStyle(currentUser).apply {
                isGroupConversation = false
                history.messages.forEach { message ->
                    addMessage(message.bodyPreview, message.createdAtMillis, sender)
                }
            }
        } else {
            @Suppress("DEPRECATION")
            android.app.Notification.MessagingStyle("Вы").apply {
                history.messages.forEach { message ->
                    addMessage(message.bodyPreview, message.createdAtMillis, history.senderNickname)
                }
            }
        }
        val intent = Intent(context, MainActivity::class.java)
            .setAction(CHEENHUB_OPEN_DIRECT_MESSAGE_ACTION)
            .putExtra(CHEENHUB_CONVERSATION_ID_EXTRA, history.conversationId)
            .addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        val pendingIntent = PendingIntent.getActivity(
            context,
            history.conversationId.hashCode(),
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = builder
            .setSmallIcon(android.R.drawable.ic_dialog_email)
            .setContentTitle(history.senderNickname)
            .setContentText(history.messages.lastOrNull()?.bodyPreview ?: "Новое сообщение")
            .setCategory(android.app.Notification.CATEGORY_MESSAGE)
            .setAutoCancel(true)
            .setOnlyAlertOnce(false)
            .setShowWhen(true)
            .setWhen(history.messages.lastOrNull()?.createdAtMillis ?: System.currentTimeMillis())
            .setContentIntent(pendingIntent)
            .setStyle(style)
            .build()
        context.getSystemService(NotificationManager::class.java).notify(
            "cheenhub_dm:${history.conversationId}",
            CONVERSATION_NOTIFICATION_ID,
            notification,
        )
    }
}

class DioxusForegroundService : Service() {
    private val activeKinds = linkedSetOf<String>()

    override fun onCreate() {
        super.onCreate()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Active calls and media",
                NotificationManager.IMPORTANCE_LOW,
            )
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val kind = intent?.getStringExtra(EXTRA_KIND) ?: return START_NOT_STICKY
        if (intent.action == ACTION_STOP) {
            activeKinds.remove(kind)
            if (activeKinds.isEmpty()) stopSelf()
            return START_NOT_STICKY
        }

        activeKinds.add(kind)
        val notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            android.app.Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            android.app.Notification.Builder(this)
        }
        val builtNotification = notification
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle("CheenHub")
            .setContentText("Active voice or media session")
            .setOngoing(true)
            .build()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, builtNotification, serviceType(kind))
        } else {
            startForeground(NOTIFICATION_ID, builtNotification)
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun serviceType(kind: String): Int = when (kind) {
        "voice" -> ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE or
            ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
        "camera" -> ServiceInfo.FOREGROUND_SERVICE_TYPE_CAMERA
        "mediaProjection" -> ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION
        else -> 0
    }

    companion object {
        const val ACTION_START = "ru.cheenhub.action.START_MEDIA_SERVICE"
        const val ACTION_STOP = "ru.cheenhub.action.STOP_MEDIA_SERVICE"
        const val EXTRA_KIND = "kind"
        private const val CHANNEL_ID = "cheenhub_active_media"
        private const val NOTIFICATION_ID = 1001
    }
}
