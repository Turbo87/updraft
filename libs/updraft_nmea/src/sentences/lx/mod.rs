//! LXNAV proprietary sentences spoken by LX80xx/LX90xx systems and the
//! LXNAV vario family (V7, S-series, Nano).

mod lxwp0;
mod lxwp1;
mod lxwp2;
mod lxwp3;

pub use lxwp0::Lxwp0;
pub use lxwp1::Lxwp1;
pub use lxwp2::Lxwp2;
pub use lxwp3::{Lxwp3, Lxwp3SpeedCommandMode, Lxwp3SwitchMode};
