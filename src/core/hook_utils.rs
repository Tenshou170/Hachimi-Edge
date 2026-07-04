use std::time::Duration;

/// Defer removing an interceptor hook to a background thread to avoid
/// calling `unhook()` while still executing inside the hooked function.
pub fn defer_unhook(addr: usize) {
    std::thread::spawn(move || {
        // Small delay to allow the hooked function to return
        std::thread::sleep(Duration::from_millis(20));
        // Only attempt to unhook if Hachimi is initialized
        if crate::core::hachimi::Hachimi::is_initialized() {
            let h = crate::core::hachimi::Hachimi::instance();
            let _ = h.interceptor.unhook(addr);
        }
    });
}
