//! Núcleo do MultiDroid, sem interface, o mesmo nos três sistemas
//! (docs/spec/MULTIPLATAFORMA.md §2). Por enquanto só os tokens de design;
//! catálogo, instalador, aparelhos, emulador e estado chegam nas etapas R-1 a R-5.

pub mod catalog;
pub mod devices;
pub mod ini;
pub mod install;
pub mod platform;
pub mod state;
pub mod tokens;
