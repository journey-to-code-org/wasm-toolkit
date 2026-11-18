//! What the page gives the engine: a speaker, and somewhere to keep a game. When the engine is
//! built for anything but the browser (the tests), these do nothing.

#[cfg(target_arch = "wasm32")]
mod raw {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn wtk_tone(hz: i32, ms: i32);
    }
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
