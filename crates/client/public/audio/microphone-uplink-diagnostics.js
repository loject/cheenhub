// Диагностика входящего PCM и ожидания отправки; политику потока не меняет.
const SEND_PROFILE_INTERVAL_MS = 5_000;

export function recordPcmInputProfile(current, frameCount, nowMs, post) {
  if (!current.diagnosticsEnabled) {
    return;
  }

  if (current.pcmProfileWindowStartMs === 0) {
    current.pcmProfileWindowStartMs = nowMs;
  }

  current.receivedPcmChunks += 1;
  current.receivedPcmFrames += frameCount;
  current.pcmProfileChunks += 1;
  current.pcmProfileFrames += frameCount;

  // Отчёт после примерно одной секунды полученного аудио, независимо от таймера.
  if (current.pcmProfileFrames < current.sampleRateHz) {
    return;
  }

  const wallDeltaMs = Math.max(0, nowMs - current.pcmProfileWindowStartMs);
  const effectiveRateHz = wallDeltaMs > 0
    ? current.pcmProfileFrames * 1000 / wallDeltaMs
    : 0;

  post("pcm-profile", {
    receivedPcmChunks: current.receivedPcmChunks,
    receivedPcmFrames: current.receivedPcmFrames,
    pcmChunksSinceReport: current.pcmProfileChunks,
    pcmFramesSinceReport: current.pcmProfileFrames,
    wallDeltaMs: Math.round(wallDeltaMs),
    effectiveRateHz,
    sampleRateHz: current.sampleRateHz,
  });

  current.pcmProfileWindowStartMs = nowMs;
  current.pcmProfileChunks = 0;
  current.pcmProfileFrames = 0;
}

export function emitSendProfile(current, nowMs, post) {
  if (!current.diagnosticsEnabled) {
    return;
  }
  const windowMs = nowMs - current.sendProfileWindowStartMs;
  if (windowMs < SEND_PROFILE_INTERVAL_MS) {
    return;
  }
  const completed = current.sendProfileCompletedCount;
  const sendWaitAvgMs = completed > 0
    ? current.sendProfileWaitTotalMs / completed
    : 0;
  const pendingSendMs = current.sendPending && current.sendStartedAtMs > 0
    ? Math.max(0, nowMs - current.sendStartedAtMs)
    : 0;
  post("send-profile", {
    transport: current.transport.kind,
    windowMs: Math.round(windowMs),
    sendCount: current.sendCount,
    sendCompletedCount: current.sendCompletedCount,
    sendBlockedCount: current.sendBlockedCount,
    sendPendingBlockedCount: current.sendPendingBlockedCount,
    sendBackpressureBlockedCount: current.sendBackpressureBlockedCount,
    sendsSinceReport: current.sendProfileSendCount,
    completedSinceReport: current.sendProfileCompletedCount,
    blockedSinceReport: current.sendProfileBlockedCount,
    pendingBlockedSinceReport: current.sendProfilePendingBlockedCount,
    backpressureBlockedSinceReport: current.sendProfileBackpressureBlockedCount,
    sendWaitTotalMs: Math.round(current.sendProfileWaitTotalMs),
    sendWaitAvgMs,
    sendWaitMaxMs: current.sendProfileWaitMaxMs,
    pendingSendMs,
    encoderQueueSize: current.encoder.encodeQueueSize,
  });
  current.sendProfileWindowStartMs = nowMs;
  current.sendProfileSendCount = 0;
  current.sendProfileCompletedCount = 0;
  current.sendProfileBlockedCount = 0;
  current.sendProfilePendingBlockedCount = 0;
  current.sendProfileBackpressureBlockedCount = 0;
  current.sendProfileWaitTotalMs = 0;
  current.sendProfileWaitMaxMs = 0;
}
