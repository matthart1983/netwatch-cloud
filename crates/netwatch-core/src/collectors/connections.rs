/// Count established TCP connections from /proc/net/tcp and /proc/net/tcp6.
/// State 01 = ESTABLISHED in the hex-encoded state field.
#[cfg(target_os = "linux")]
pub fn count_established_connections() -> u32 {
    let count_file = |path: &str| -> u32 {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return 0;
        };
        contents
            .lines()
            .skip(1) // header
            .filter(|line| {
                line.split_whitespace()
                    .nth(3)
                    .map(|st| st == "01")
                    .unwrap_or(false)
            })
            .count() as u32
    };

    count_file("/proc/net/tcp") + count_file("/proc/net/tcp6")
}

#[cfg(not(target_os = "linux"))]
pub fn count_established_connections() -> u32 {
    0
}
