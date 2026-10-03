//! Глобальные настройки хоста CheenHub.

mod activity_chart;
mod activity_panel;
pub(crate) mod api;
mod chart_bridge;
mod dashboard;
mod log_stream;
mod logs_page;
mod messages_chart;
mod page;
mod settings_page;
mod stats_panel;
mod tabs;

pub(crate) use dashboard::HostDashboardPage;
pub(crate) use logs_page::HostLogsPage;
pub(crate) use page::HostEmailSettingsPage;
pub(crate) use settings_page::HostSystemSettingsPage;
