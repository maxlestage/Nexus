//! Cœur de l'application iPhone, en Rust.
//!
//! Tout ce qui n'est pas imposé par Apple vit ici : l'état, les actions, les
//! libellés, la description de l'interface et le protocole Bluetooth. Swift
//! ne conserve que deux responsabilités qu'aucun autre langage ne peut
//! assumer sur iOS — parler à CoreBluetooth, et dessiner à l'écran ce que ce
//! module décrit.
//!
//! Conséquence pratique : l'interface entière se teste ici, sans iPhone.

pub mod ffi;
pub mod labels;
pub mod state;
pub mod view;

pub use state::{AppState, BleEvent};
pub use view::View;
