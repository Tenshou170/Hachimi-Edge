#![allow(
    function_casts_as_integer,
    static_mut_refs,
    non_snake_case,
    non_camel_case_types
)]

#[cfg(target_os = "linux")]
compile_error!(
    "Linux targets are not supported by Hachimi Edge. Use `cargo xcheck`/`cargo xbuild` for Windows MSVC via cargo-xwin, or `cargo acheck`/`cargo abuild` for Android."
);

#[cfg(all(target_os = "windows", target_env = "gnu"))]
compile_error!(
    "Windows GNU/MinGW targets are not supported by Hachimi Edge. Use `cargo xcheck`/`cargo xbuild` for the Windows MSVC target."
);

#[macro_use]
extern crate log;
#[macro_use]
extern crate cstr;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Explicit version marker for external tooling (e.g. UmaPatcher).
/// The byte sequence "HACHIMI_VERSION:" followed immediately by the version string
/// is guaranteed to be present in the .so binary. Do NOT remove or rename this.
#[used]
#[no_mangle]
static HACHIMI_VERSION_MARKER: &str =
    concat!("HACHIMI_VERSION:", env!("HACHIMI_DISPLAY_VERSION"));

rust_i18n::i18n!("assets/locales", fallback = "en");

#[macro_use]
pub mod core;
pub mod il2cpp;

/** Android **/
#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
use android::{game_impl, gui_impl, hachimi_impl, interceptor_impl, log_impl, symbols_impl};

/** Windows **/
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
use windows::{game_impl, gui_impl, hachimi_impl, interceptor_impl, log_impl, symbols_impl};
