// Please keep the playdate crate root's comment and README.md in sync.

//! # Playdate
//! 
//! This crate and its related crates together provide a safe Rust API for the
//! [Playdate](https://play.date/) hand held gaming system.
//! 
//! # Requirements
//! Using these crates requires the [Playdate SDK](https://play.date/dev/), which has [its own
//! license](https://play.date/dev/sdk-license). Install the SDK and add an environment variable
//! named `PLAYDATE_SDK_PATH` that points to the directory where you installed it, such as
//! `PLAYDATE_SDK_PATH=C:\playdate`.
//! 
//! This crate uses unstable features in order to provide a #[no_std] library and application to run
//! on the Playdate simulator and hardware devices. Thus it requires use of the [Rust nightly
//! compiler](https://doc.rust-lang.org/1.2.0/book/nightly-rust.html).
//! 
//! # Getting Started
//! 
//! Building a #[no_std] application that is compiled for the Playdate simulator requires a bit of
//! extra work and Cargo setup. The dependency structure of your project will look like this:
//! 
//! ```
//! - your-game-project** (crate of type "cdylib")
//!   ├── [dependencies] your-game** (#[no_std] crate of type "rlib")
//!   |   ├── [dependencies] playdate (#[no_std] crate of type "rlib")
//!   |   └── [dependencies] euclid (with `default-features = false` to keep it compatible with #[no_std]) (used in the playdate API)
//!   └── [build-dependencies] playdate-build
//! 
//! ** = is specific to your game and provided by the game developer.
//! ```
//! 
//! ## The root project crate
//! 
//! We provide an template root project crate at [playdate-project](TODO: link), which will act as
//! the coordination point to build your game for the Playdate simulator and the Playdate device. To
//! use it, please rename and customize it for your game.
//! 
//! To start using it, download the latest release, unzip it and edit it as follows. See below for
//! more details:
//! 1. Ensure your `PLAYDATE_SDK_PATH` environment variable is set to the location of the Playdate
//!    SDK.
//! 1. In the `Cargo.toml` file, change the `name` to include your game's name.
//! 1. In the `Cargo.toml` file, change the `game` dependency's `package` and `path` to point to
//!    your game's crate.
//! 1. In the `Cargo.toml` file, remove or change the `game-assets` dependency's `package` and
//!    `path` to point to your game's asset-generating crate.
//! 1. If you kept the `game-assets` dependency for generating assets, call it from
//!    `src/bin/make_pdx.rs`.
//! 
//! If you choose not to use the template crate, then you do not need the [playdate-build](TODO:
//! link) crate listed in `[build-dependencies]`.
//! 
//! ### Development Workflow
//! 
//! To build your game for the Playdate simulator, simply build your customized root project
//! `your-game-project` crate with the Cargo `--lib` flag, which will build your game as a
//! dependency.
//! 
//! After building the game, the root project crate (if based on [playdate-project](TODO: link))
//! includes 2 binaries to help you get it onto the Playdate simulator or a hardware device. Build
//! them by building your root project `your-game-project` crate with the Cargo `--bins` flag. The
//! binaries are:
//! * make_pdx
//! * run_simulator
//! 
//! #### make_pdx
//! Combines your built game, along with any asset files into a pdx image for the device or
//! simulator, which is found in `$OUT_DIR/pdx_out`.
//! 
//! The `your-game-assets` dependency seen above is an optional place to construct and collect
//! assets for your game that will be included by **make_pdx** when building the game's pdx image.
//! To do so, edit the `make_pdx.rs` file to call `your-game-assets`. Assets should be collected
//! into `env!("PDX_SOURCE_DIR")`. For example:
//! ```rs
//!   your_game_assets::generate_assets(env!("PDX_SOURCE_DIR"))?;
//! ```
//! 
//! The **make_pdx** binary would then include those assets into your game's pdx image.
//! 
//! #### run_simulator
//! 
//! Runs the Playdate simulator, loading the pdx image generated by **make_pdx**.
//! 
//! #### VSCode
//! 
//! If you're using VSCode, the template root project [playdate-project](TODO: link) crate comes
//! with two files to provide tasks that build your game's pdx image and run it on the Windows
//! simulator.
//! * `.vscode/settings.json`: The `"projectRootCrate"` variable should point to the root project
//!   crate. By default, since the `.vscode` directory is inside that crate, it is `"."`. Similarly,
//!   the `"rust-analyzer.linkedProjects"` variable should point to the root project crate's
//!   `Cargo.toml` file. By default it is `"./Cargo.toml"`.
//! * `.vscode/tasks.json`: Provides the tasks to build a pdx for the Playdate simulator, and to
//!   load it into the simulator, or to build a pdx for the Playdate device.
//! 
//! When running the simulator with this task, VSCode will capture the `stdout` and `stderr` output
//! of the game and write it to a file called `stdout.txt`.
//! 
//! ### Panics
//! 
//! The `Cargo.toml` for the root project crate must also set `panic = "abort"`. This is included in
//! the template root project [playdate-project](TODO: link) crate:
//! ```
//! [profile.dev]
//! panic = "abort"
//! [profile.release]
//! panic = "abort"
//! ```
//! Otherwise you will get a compilation error:
//! ```
//! error: language item required, but not found: `eh_personality`
//!   |
//!   = note: this can occur when a binary crate with `#![no_std]` is compiled for a target where `eh_personality` is defined in the standard library
//! ```
//! 
//! ## Your first game
//! 
//! Your game's crate must include a function that will be called after the Playdate system
//! initializes. This function should contain your game's main game loop. It's simplest form would
//! look like:
//! ```rs
//! #[playdate::main]
//! async fn main(api: playdate::Api) -> ! {
//!   let events = api.system.system_event_watcher();
//!   loop {
//!     match events.next().await {
//!       playdate::SystemEvent::NextFrame { inputs, .. } => {
//!         // Read inputs, update game state and draw.
//!       }
//!       _ => (),
//!     }
//!   }
//! }
//! ```
//! Then, handle the various events that can be returned from `next()`. In particular, handle input,
//! update game state, and draw to the screen when the `SystemEvent::NextFrame` event happens. The
//! Playdate system APIs are available throught the `playdate::Api` parameter to `main()`.
//! 
//! Logging to the simulator's console for debugging is possible through the `playdate::log()` and
//! `playdate::log_error()` functions.
//! 
//! # Platforms
//! 
//! **Currently this project only supports development for the Windows simulator.** We will expand
//! support to the Playdate hardware device once we get access to one. Simulators on other platforms
//! (e.g. Mac) are possible, and would only need changes to the root project crate.
//! 
//! # License
//! This project is licensed under either of
//! 
//! * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
//!   https://www.apache.org/licenses/LICENSE-2.0)
//! * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
//! 
//! at your option.
//! 
//! ## Contribution
//! Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
//! playdate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
//! any additional terms or conditions.

#![no_std]
#![deny(clippy::all)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(never_type)]

extern crate alloc;
extern crate playdate_macro;

/// A game crate should annotate their game loop function with this attribute macro.
///
/// The annotated function must be async, and will indicate that it's done updating
/// and ready to draw by `await`ing the `draw` Future passed to it.
pub use playdate_macro::main;

mod allocator;
mod api;
mod callback_builder;
mod callbacks;
mod capi_state;
mod clamped_float;
mod ctypes;
mod ctypes_enums;
mod display;
mod error;
mod executor;
mod files;
mod geometry;
mod graphics;
mod inputs;
mod log;
mod menu;
mod null_terminated;
mod sound;
mod system;
mod system_event;
mod time;

#[doc(hidden)]
pub mod macro_helpers;

/// Reexport some of alloc, since things in alloc are not guaranteed to work in `no_std` as it all
/// depends on our global allocator. This makes it clear they can be used, and avoids the need for
/// `extern crate alloc` elsewhere.
pub use alloc::{borrow::ToOwned, format, string::String};

pub use api::*;
pub use callback_builder::{CallbackBuilder, CallbackBuilderWithArg};
pub use callbacks::Callbacks;
pub use clamped_float::*;
pub use ctypes_enums::*;
pub use display::*;
pub use error::*;
pub use files::*;
pub use geometry::*;
pub use graphics::*;
pub use inputs::*;
pub use log::{log, log_error};
pub use menu::*;
pub use sound::*;
pub use system::*;
pub use system_event::*;
pub use time::*;

/// The global allocator, which will defer allocation requests to the playdate system, and deal with
/// ensuring correct alignment.
#[global_allocator]
static mut GLOBAL_ALLOCATOR: allocator::Allocator = allocator::Allocator::new();

/// A helper implementation of panic_handler for the toplevel crate to forward to.
///
/// Since the top-level crate has to implement the `#[panic_handler]` we make it
/// easy by letting them simply forward over to this function.
#[cfg(not(target_arch = "arm"))]
pub fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
  crate::log::log_to_stdout("panic!");
  if let Some(loc) = panic_info.location() {
    crate::log::log_to_stdout(" at ");
    crate::log::log_to_stdout(loc.file());
    crate::log::log_to_stdout(":");
    crate::log::log_usize_to_stdout(loc.line() as usize);
    crate::log::log_to_stdout(":");
    crate::log::log_usize_to_stdout(loc.column() as usize);

    // TODO: caller()s.

    crate::log::log_to_stdout_with_newline("");
  }

  if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
    crate::log::log_to_stdout("payload: ");
    crate::log::log_to_stdout(s);
    crate::log::log_to_stdout("\n");
  } else {
    //crate::debug::log_bytes_to_stdout(b"panic has unknown payload");
  }

  core::intrinsics::abort()
}

#[doc(hidden)]
#[cfg(target_arch = "arm")]
pub fn panic_handler(_panic_info: &core::panic::PanicInfo) -> ! {
  core::intrinsics::abort()
}

/// The error handler for when allocations fail. It will simply panic.
#[alloc_error_handler]
fn playdate_alloc_error_handler(layout: core::alloc::Layout) -> ! {
  panic!(
    "memory allocation of {} bytes at alignment {} failed",
    layout.size(),
    layout.align()
  )
}

/// A way to store a pointer in a static variable, by telling the compiler it's Sync.
///
/// This is, of course, unsound if the pointer is used across threads and is not
/// thread-safe, but the pointer is only used by the Playdate system.
#[repr(transparent)]
struct BssPtr(*const u32);
unsafe impl Sync for BssPtr {}

extern "C" {
  static __bss_start__: u32;
  static __bss_end__: u32;
}

#[cfg(target_arch = "arm")]
#[used]
#[link_section = ".bss_start"]
static BSS_START_PTR: BssPtr = unsafe { BssPtr(&__bss_start__) };

#[cfg(target_arch = "arm")]
#[used]
#[link_section = ".bss_end"]
static BSS_END_PTR: BssPtr = unsafe { BssPtr(&__bss_end__) };
