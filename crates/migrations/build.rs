//! Проверки сборки миграций во время компиляции.

mod file_lines {
    include!("../../build_support/file_lines.rs");
}

fn main() {
    file_lines::check_workspace_file_lines();
}
