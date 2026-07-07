#![warn(missing_docs)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
//! Точка входа установщика CheenHub с пользовательским UI.

mod installer;

fn main() {
    installer::run();
}
