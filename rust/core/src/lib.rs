//! Núcleo do MultiDroid, sem interface, o mesmo nos três sistemas
//! (docs/spec/MULTIPLATAFORMA.md §2). Por enquanto só os tokens de design;
//! catálogo, instalador, aparelhos, emulador e estado chegam nas etapas R-1 a R-5.

pub mod adb;
pub mod catalog;
pub mod devices;
pub mod emulator;
pub mod idioma;
pub mod ini;
pub mod install;
pub mod leve;
pub mod native;
pub mod platform;
pub mod scrcpy;
pub mod screen;
pub mod state;
pub mod studio;
pub mod tokens;
