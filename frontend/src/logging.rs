use log::Level;

/// Initialize the browser console logging just once.
pub fn init_logging() {
    let _ = console_log::init_with_level(Level::Debug);
}
