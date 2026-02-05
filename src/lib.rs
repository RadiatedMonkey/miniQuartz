#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod utilities;
pub mod playback;
pub mod playlist;
pub mod UI;

pub use app::TemplateApp;
