
pub(super) fn timespec(duration: std::time::Duration) -> io_uring::types::Timespec {
    io_uring::types::Timespec::new()
        .sec(duration.as_secs())
        .nsec(duration.subsec_nanos())
}