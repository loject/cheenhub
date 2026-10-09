# Windows Push-to-talk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Передавать голос при удержании выбранной клавиши вне фокуса Windows-приложения; показывать недоступность глобального Push-to-talk на остальных платформах.

**Architecture:** Режим и клавиша принадлежат microphone feature. Платформенный контракт проверяет удержание клавиши; Windows использует WH_KEYBOARD_LL на отдельном event loop, web и unsupported возвращают недоступность. Gate применяется до кодирования; существующие mute, capture lifecycle и realtime adapters сохраняют ответственность.

**Tech Stack:** Rust, Dioxus signals/events, dioxus-sdk-storage, windows-sys, существующий Opus encoder.

**Spec:** Запрос пользователя в текущем чате и AGENTS.md.

## Global Constraints

- Глобальный Push-to-talk поддерживается только Windows desktop.
- Platform cfg и Windows API находятся только в platform implementation files.
- Новые комментарии и документация на русском; production files не превышают 500 строк.
- Нажатие не включает микрофон после ручного mute и не создаёт voice session вне звонка.
- Не добавлять JS/browser API, новые realtime методы или подавления lint.

## Review Focus

- Отпускание клавиши закрывает gate без задержки VAD.
- Смена фокуса не меняет режим и не блокирует чтение выбранной клавиши.
- Ручной mute и выход из звонка запрещают передачу даже при удержании.
- Сохранённый Windows PTT не вызывает передачу на unsupported платформе.
- Невалидная сохранённая клавиша заменяется безопасным значением; смена привязки не оставляет gate открытым.

## Approved tradeoff

Пользователь выбрал keyboard hook с отдельным event loop и восстановлением. GetAsyncKeyState используется только watchdog для обнаружения рассинхронизации; обычные нажатия/отпускания приходят через hook. Heartbeat закрывает gate при зависании event loop.

Устройство захвата остаётся открытым во время активного звонка; при отпущенной клавише звук не кодируется и не передаётся. Это исключает задержку запуска устройства при каждом нажатии.

### Task 1: Платформенный контракт и gate

**Files:**
- Create: `crates/client/src/features/microphone/push_to_talk/backend.rs`, `platform.rs`, `platform/windows.rs`, `platform/web.rs`, `platform/unsupported.rs`, `key.rs`, `key/tests.rs`.
- Modify: `microphone/backend.rs`, `microphone/vad.rs`, `microphone/vad/tests.rs`, `microphone/native/encoding.rs`, `microphone/mod.rs`, `crates/client/Cargo.toml`.

**Interfaces:** `PushToTalkKey` хранит проверенный выбор клавиши; `supported() -> bool`, `unsupported_reason() -> Option<&'static str>`, `is_pressed(key: PushToTalkKey) -> bool`. `MicrophoneConfig` получает клавишу; `MicrophoneActivationMode::PushToTalk` открывает gate только при удержании и обходит RMS-порог.

- [ ] Добавить тесты выбора клавиши: roundtrip, неизвестное значение, значение по умолчанию.
- [ ] Добавить gate-тесты: отпущено + громкий PCM -> false; удержано + тихий PCM -> true; отпускание -> false немедленно; unsupported -> false.
- [ ] Запустить узкие тесты и подтвердить отсутствие требуемого поведения.
- [ ] Реализовать контракт, Windows keyboard hook с отдельным event loop и watchdog и gate; добавить `Win32_UI_Input_KeyboardAndMouse` к windows-sys.
- [ ] Проверить все matches режима, включая browser_worker: неподдерживаемый PTT закрывает gate; не меняет browser wire protocol без необходимости.
- [ ] Запустить тесты gate и существующие VAD-тесты.

### Task 2: Настройки и жизненный цикл

**Files:**
- Modify: `microphone/storage.rs`, `microphone/provider.rs`, `microphone/provider_context.rs`, `microphone/provider_preferences.rs`, `microphone/provider_runtime.rs`, `microphone/provider/tests.rs`.
- Create: `microphone/push_to_talk/preferences.rs`, `microphone/push_to_talk/preferences/tests.rs`, `crates/client/src/features/user_settings/push_to_talk_settings.rs`.
- Modify: `user_settings/mod.rs`, `user_settings/sound_section.rs`.

**Interfaces:** `MicrophoneHandle` предоставляет выбранную клавишу и её обновление; сохранение использует существующий LocalStorage. Отдельный компонент рисует Windows-настройку клавиши и видимую причину недоступности на других платформах.

- [ ] Добавить тесты сохранённого режима/ключа, unsupported normalization, конфигурации capture и stop ownership.
- [ ] Проверить падение тестов до реализации.
- [ ] Реализовать хранение/сигналы и передачу настройки в capture config, пользуясь существующим restart при смене настроек.
- [ ] По умолчанию назначить правый Ctrl; предложить выбор из Ctrl/Alt/Shift/Space/F1–F12 с явной меткой стороны модификатора.
- [ ] Заменить «в разработке» доступным Windows режимом и компонентом настройки. Web: «Глобальный Push-to-talk недоступен в браузере. Используйте приложение для Windows». Linux/прочие: «Глобальный Push-to-talk доступен в приложении для Windows».
- [ ] Добавить debug-логи переходов gate, info изменения режима/клавиши и actionable failures без содержимого аудио.
- [ ] Подтвердить mute/leave/startup race тестами в owning feature; не переносить управление звонком в microphone.

### Task 3: Проверка и передача

- [ ] Запустить `cargo fmt` и `cargo clippy --workspace --all-targets`.
- [ ] Запустить `cargo test --workspace --all-targets`, поскольку меняется протестированное поведение микрофона.
- [ ] Запустить client desktop Windows проверки с `--no-default-features --features desktop,windows`; проверить web с `--features web` и доступным wasm target.
- [ ] Проверить UI-копирайт, число компонентов и лимит строк изменённых production files.
- [ ] По возможности проверить вручную удержание/отпускание при фокусе в другом приложении; иначе явно указать непроверенный сценарий.
- [ ] Сообщить результат, проверки и последствия tradeoffs; не создавать commit без запроса.


## Execution notes

- Ruling: реализация выполняется в `codex/windows-push-to-talk`, без commit/push.
- Gate и клавиши: изолированные RED→GREEN тесты удержания/восстановления, затем проверки в настоящем Windows module tree.
- Независимое review обнаружило backlog PCM и синхронные логи в hook callback: исправлены capture gate/hold epochs и минимальный callback.
- Дополнительный RED→GREEN regression: первый device buffer после смены удержания отбрасывается; partial PCM и queued encoded frames предыдущего удержания не отправляются.
- Capture остаётся открытым во время звонка. Первый buffer отбрасывается ради исключения звука до нажатия; ввод не поглощается другими приложениями.
- Workspace test suite прошла: 718 тестов, 6 ignored. Windows full suite: 399 passed, 2 failures в `features::toast::provider::tests::ordinary_messages_do_not_evict_update_notification` и `features::toast::provider::tests::refresh_preserves_notification_identity_and_selected_deferral`: тестовый VirtualDom не предоставляет DesktopService для ApplicationFocusProvider. Эти файлы не изменялись; microphone тесты проверяются отдельно.
- Проверка реального удержания во время звонка в другом приложении не выполнена; Linux toolchain не установлен. Web wasm check и Windows clippy выполняются.


## Final verification

- `cargo test --workspace --all-targets`: 719 passed, 6 ignored, exit 0.
- Windows microphone suite: 47 passed, exit 0.
- `cargo fmt --check`: exit 0.
- `cargo clippy --workspace --all-targets`: exit 0, только существующие предупреждения.
- Windows client Clippy: завершён успешно, без новых предупреждений в изменённом коде.
- Независимое итоговое review: все замечания исправлены, новых blockers нет.
- EncodedMicrophoneFrame содержит локальное отзываемое разрешение; provider queue и voice adapter проверяют его до передачи. Wire format и generic realtime не изменены.

## Tradeoffs and consequences

- Hook с отдельным event loop и watchdog сложнее опроса, зато обычный ввод событийный; watchdog проверяет выбранную клавишу каждые 250 мс и восстанавливает потерянный hook.
- Устройство захвата открыто во время звонка: старт удержания не ждёт запуска устройства, но системный индикатор микрофона остаётся включённым.
- Первый device buffer после нового удержания отбрасывается, чтобы не передать звук до нажатия: начало речи задерживается до одного буфера устройства.
- Назначенная клавиша не поглощается: другие приложения продолжают реагировать на неё.
- Разрешение сопровождает кадры через локальные очереди: небольшой дополнительный расход памяти исключает отправку отозванных кадров. Уже переданные транспорту datagrams не отзываются.
- Linux build не выполнялся: Linux target/toolchain отсутствует. Сценарий реального звонка вне фокуса требует ручной проверки.
- Web wasm build: cargo check с wasm32-unknown-unknown и features web завершён успешно.

## Расширение назначения кнопки и исправление UI

- Выбор из списка заменён записью произвольной клавиши Windows и Mouse1–Mouse5. Привязка сохраняется после отпускания; стороны модификаторов различаются.
- Запись использует отдельные keyboard/mouse hooks, освобождает их при отмене и ограничивает ожидание 30 секундами. Удерживаемые до начала записи кнопки пропускаются до отпускания; regression-тест покрывает autorepeat.
- На время назначения PTT приостанавливается. Поколение назначения отзывает уже подготовленные кадры; отмена future восстанавливает передачу через RAII.
- Карточки режимов используют единое оформление и размер. Назначение вынесено ниже карточек, включает индикатор ожидания, отмену и сообщение ошибки.
- Независимое review подтвердило исправление pre-held autorepeat; других подтверждённых blockers нет.

### Дополнительные компромиссы

- Привязка содержит одну удерживаемую кнопку, без сочетаний; колесо не имеет состояния удержания и не записывается.
- Во время назначения выбранные keyboard/боковые mouse события временно поглощаются, чтобы избежать shortcuts и перехода назад. Mouse1/Mouse2 пропускаются для доступной отмены и могут выполнять обычный клик. В обычном режиме PTT события не поглощаются.
- Кнопки Mouse6 и выше доступны через клавиатурное переназначение в драйвере мыши, если драйвер не представляет их как Mouse4/Mouse5.
- Запись завершается отпусканием, чтобы назначенная кнопка сразу не открывала передачу. PTT приостановлен до завершения или отмены назначения.
- Реальный звонок вне фокуса и визуальная проверка Windows UI не выполнены; автоматические тесты проверяют правила и сборку, но не работу конкретного драйвера ввода.

### Проверки расширения

- Windows microphone tests: 55 passed, exit 0.
- Workspace tests: 721 passed, 6 ignored, exit 0.
- cargo fmt --check: exit 0.
- Workspace и Windows Clippy: exit 0, без новых предупреждений в реализации.
- Web wasm check: exit 0.
- Изменённые production Rust-файлы не превышают 500 строк; git diff --check прошёл.
