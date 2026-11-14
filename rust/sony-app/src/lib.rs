//! Qt-free application core shared by the settings window and the bar widget:
//! device connection, state snapshot, commands and the worker thread.

pub mod backend;
pub mod device_core;

pub use backend::{Backend, Emit, PairedDevice, Request, TransportFactory, Update};
pub use device_core::{Caps, Command, Link, Snapshot};
