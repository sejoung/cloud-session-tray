use std::ffi::OsString;

pub(super) fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

pub(super) fn profile_option(key: &str, value: &str) -> OsString {
    format!("--{key}={value}").into()
}

pub(super) fn short_network_command(command: &[&str], profile: &str) -> Vec<OsString> {
    let mut arguments = args(command);
    arguments.extend(args(&[
        "--no-cli-pager",
        "--cli-connect-timeout",
        "8",
        "--cli-read-timeout",
        "12",
    ]));
    arguments.push(profile_option("profile", profile));
    arguments
}
