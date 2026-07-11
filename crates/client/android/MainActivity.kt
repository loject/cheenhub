package dev.dioxus.main

import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
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
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.util.Range
import android.view.Surface
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

typealias BuildConfig = ru.cheenhub.BuildConfig

class MainActivity : WryActivity() {
    private val captures = ConcurrentHashMap<Int, CaptureResources>()
    private var voiceAudioFocus: AudioFocusRequest? = null

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
