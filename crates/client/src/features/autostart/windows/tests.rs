use std::ffi::OsString;
use std::path::Path;

use super::command_for_executable;

#[test]
fn startup_command_quotes_executable_path() {
    assert_eq!(
        command_for_executable(Path::new(r"C:\Program Files\CheenHub\cheenhub.exe")),
        OsString::from(r#""C:\Program Files\CheenHub\cheenhub.exe" --startup-hidden"#)
    );
}
