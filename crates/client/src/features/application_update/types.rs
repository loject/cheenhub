//! Типы состояния проверки и скачивания обновлений.

/// Asset GitHub Release, подходящий для скачивания обновления.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UpdateDownloadAsset {
    /// Имя файла в GitHub Release.
    pub(crate) name: String,
    /// Прямая ссылка на скачивание asset'а.
    pub(crate) download_url: String,
    /// Размер файла в байтах.
    pub(crate) size_bytes: u64,
}

/// Скачанный файл обновления.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadedUpdate {
    /// Имя сохраненного файла.
    pub(crate) file_name: String,
    /// Человекочитаемый путь к сохраненному файлу.
    pub(crate) path: String,
}

/// Прогресс скачивания файла обновления.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UpdateDownloadProgress {
    /// Количество уже скачанных байт.
    pub(crate) downloaded_bytes: u64,
    /// Общий размер файла в байтах, если он известен.
    pub(crate) total_bytes: Option<u64>,
    /// Текущая средняя скорость скачивания в байтах в секунду.
    pub(crate) bytes_per_second: u64,
}

/// Найденный GitHub Release, который новее текущей версии приложения.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AvailableUpdate {
    /// Версия релиза без префикса `v`.
    pub(crate) version: String,
    /// Исходный Git tag релиза.
    pub(crate) tag: String,
    /// Человекочитаемый заголовок релиза.
    pub(crate) title: Option<String>,
    /// Страница релиза на GitHub.
    pub(crate) release_url: String,
    /// Установщик для текущей платформы, если он опубликован в релизе.
    pub(crate) download_asset: Option<UpdateDownloadAsset>,
}

/// Состояние скачивания обновления.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UpdateDownloadStatus {
    /// Скачивание еще не запускалось.
    Idle,
    /// Файл обновления скачивается.
    Downloading {
        /// Версия скачиваемого обновления.
        version: String,
        /// Прогресс скачивания файла.
        progress: UpdateDownloadProgress,
    },
    /// Файл обновления сохранен локально.
    Downloaded {
        /// Версия скачанного обновления.
        version: String,
        /// Данные сохраненного файла.
        file: DownloadedUpdate,
    },
    /// Скачивание завершилось ошибкой.
    Failed {
        /// Версия обновления, для которой скачивание завершилось ошибкой.
        version: String,
        /// Сообщение об ошибке для пользователя.
        message: String,
    },
}
