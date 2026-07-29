use aviutl2::tracing;

static BUFFERS: std::sync::LazyLock<dashmap::DashMap<usize, Vec<u8>>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

pub fn lend_buffer(buffer: Vec<u8>) -> (*const u8, usize) {
    let ptr = buffer.as_ptr();
    let len = buffer.len();
    BUFFERS.insert(ptr as usize, buffer);
    tracing::debug!("lend_buffer called, buffer length: {}, ptr: {:?}", len, ptr);
    (ptr, len)
}

pub fn release_buffer(ptr: *const u8) -> bool {
    if BUFFERS.remove(&(ptr as usize)).is_some() {
        tracing::debug!("release_buffer called, ptr: {:?}, buffer released", ptr);
        true
    } else {
        tracing::warn!("release_buffer called, ptr: {:?}, buffer not found", ptr);
        false
    }
}
