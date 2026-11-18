//! An engine for driving games of the old kind: a 320x240 screen in 1,024 colours, a pointer, and
//! places made of pictures. It plays whatever content pack it is given (`pack`), and has no cars,
//! rooms or words of its own.
//!
//! To the page it looks like the C toolkit did, so the same host runs it: `wtk_init`, `wtk_input`,
//! `wtk_step`, `wtk_screen`, `wtk_palette`. Before `wtk_init` the page hands over a pack with
//! `pack_alloc` and `pack_load`.

pub mod font;
pub mod g3d;
pub mod game;
pub mod gfx;
pub mod host;
pub mod pack;
pub mod race;
pub mod road;
pub mod town;

use game::Game;
use gfx::Screen;

struct Engine {
    screen: Screen,
    game: Game,
    keys: u32,
    mouse: (i32, i32),
    pack_bytes: Vec<u8>,
    /// Whether the page has been asked for a line to the meet.
    line: bool,
}

static mut ENGINE: Option<Engine> = None;
static mut PACK_BUFFER: Vec<u8> = Vec::new();
static mut NET_BUFFER: Vec<u8> = Vec::new();

#[allow(static_mut_refs)]
fn engine() -> &'static mut Engine {
    // The page calls in on one thread, one call at a time.
    unsafe {
        ENGINE.get_or_insert_with(|| Engine {
            screen: Screen::new(),
            game: Game::new(Err("no pack was loaded".into())),
            keys: 0,
            mouse: (160, 100),
            pack_bytes: Vec::new(),
            line: false,
        })
    }
}

/// Room for a pack of `n` bytes. The page copies the pack in, then calls `pack_load`.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn pack_alloc(n: u32) -> *mut u8 {
    unsafe {
        PACK_BUFFER = vec![0; n as usize];
        PACK_BUFFER.as_mut_ptr()
    }
}

/// Take the pack that was copied in. Returns 1 if it can be played; if not, the screen says why.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn pack_load() -> i32 {
    let bytes = unsafe { core::mem::take(&mut PACK_BUFFER) };
    let e = engine();
    e.pack_bytes = bytes;
    e.game = Game::new(pack::Pack::parse(&e.pack_bytes));
    e.game.problem.is_none() as i32
}

#[no_mangle]
pub extern "C" fn wtk_init(seed: u32) {
    let e = engine();
    e.game = Game::new(pack::Pack::parse(&e.pack_bytes));
    e.game.seed(seed);
    e.game.draw(&mut e.screen);
}

#[no_mangle]
pub extern "C" fn wtk_input(keys: u32, mx: i32, my: i32) {
    let e = engine();
    e.keys = keys;
    e.mouse = (mx.clamp(0, gfx::W - 1), my.clamp(0, gfx::H - 1));
}

#[no_mangle]
pub extern "C" fn wtk_step() {
    let e = engine();
    e.game.tick(e.keys, e.mouse.0, e.mouse.1);
    e.game.draw(&mut e.screen);
    talk(e);
}

/// What the game wants of the line to the meet, passed on to the page: open or shut, and what to send.
fn talk(e: &mut Engine) {
    if e.game.net.want != e.line {
        e.line = e.game.net.want;
        host::net_line(e.line);
    }
    for message in e.game.net.outbox.drain(..) {
        if e.line {
            host::net_send(&message);
        }
    }
}

/// Room for a message of `n` bytes from the meet. The page copies it in, then calls `net_recv`.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn net_alloc(n: u32) -> *mut u8 {
    unsafe {
        NET_BUFFER = vec![0; n.min(4096) as usize];
        NET_BUFFER.as_mut_ptr()
    }
}

#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn net_recv() {
    let bytes = unsafe { core::mem::take(&mut NET_BUFFER) };
    if let Ok(text) = core::str::from_utf8(&bytes) {
        let e = engine();
        e.game.net_in(text);
        talk(e);
    }
}

/// How the line is: 2 open, 3 not to be had or dropped.
#[no_mangle]
pub extern "C" fn net_status(code: u32) {
    let e = engine();
    e.game.net_status(code as u8);
    talk(e);
}

/// What the game is doing, for a page that shows buttons on a touch screen: 0 the title, 1 pointing
/// and picking, 2 on the strip, 3 driving round a map, 4 over.
#[no_mangle]
pub extern "C" fn wtk_mode() -> i32 {
    match engine().game.scene {
        game::Scene::Title => 0,
        game::Scene::Race => 2,
        game::Scene::Drive | game::Scene::Road => 3,
        game::Scene::Over(_) => 4,
        _ => 1,
    }
}

#[no_mangle]
pub extern "C" fn wtk_screen() -> *const u16 {
    engine().screen.fb.as_ptr()
}

#[no_mangle]
pub extern "C" fn wtk_palette() -> *const u32 {
    engine().screen.pal.as_ptr()
}
