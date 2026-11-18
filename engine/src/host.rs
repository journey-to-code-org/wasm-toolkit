//! What the page gives the engine: a speaker, and somewhere to keep a game. When the engine is
//! built for anything but the browser (the tests), the speaker does nothing and the game is kept
//! in memory.

#[cfg(target_arch = "wasm32")]
mod raw {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn wtk_tone(hz: i32, ms: i32);
        pub fn wtk_store(ptr: *const u8, n: i32);
        pub fn wtk_fetch(ptr: *mut u8, max: i32) -> i32;
        pub fn wtk_net_open();
        pub fn wtk_net_close();
        pub fn wtk_net_send(ptr: *const u8, n: i32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static KEPT: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A tone from the speaker: a square wave of `hz` for `ms`.
pub fn tone(hz: i32, ms: i32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::wtk_tone(hz, ms)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (hz, ms);
}

/// Keep these bytes until next time.
pub fn store(bytes: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::wtk_store(bytes.as_ptr(), bytes.len() as i32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    KEPT.with(|k| *k.borrow_mut() = bytes.to_vec());
}

/// What was kept, or nothing.
pub fn fetch() -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = vec![0u8; 16384];
        let n = unsafe { raw::wtk_fetch(buf.as_mut_ptr(), buf.len() as i32) };
        buf.truncate(n.clamp(0, 16384) as usize);
        buf
    }
    #[cfg(not(target_arch = "wasm32"))]
    KEPT.with(|k| k.borrow().clone())
}

/// Ask the page for a line to the meet, or to shut it. How that goes comes back through `net_status`.
pub fn net_line(open: bool) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        if open { raw::wtk_net_open() } else { raw::wtk_net_close() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = open;
}

/// A message for the others in the meet.
pub fn net_send(text: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        raw::wtk_net_send(text.as_ptr(), text.len() as i32)
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = text;
}
