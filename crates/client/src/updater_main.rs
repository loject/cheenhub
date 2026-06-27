#![warn(missing_docs)]
//! Точка входа отдельного desktop-апдейтера CheenHub.

mod updater;

fn main() {
    updater::run();
}
