//! FLARM proprietary sentences: the collision-warning heartbeat
//! (`PFLAU`), proximate traffic (`PFLAA`), and configuration exchange
//! (`PFLAC`), supported debug content (`PFLAL`), and version information
//! (`PFLAV`), per the FLARM data port ICD (FTD-012), plus periodic identity
//! data from the FLARM Messaging ICD (FTD-109).

mod common;
mod pflaa;
mod pflac;
mod pflal;
mod pflam;
mod pflau;
mod pflav;

pub use common::{FlarmAlarmLevel, FlarmId};
pub use pflaa::{FlarmAircraftType, FlarmIdType, FlarmSource, Pflaa};
pub use pflac::{Pflac, PflacQueryType};
pub use pflal::{Pflal, PflalConfiguration, PflalContent, PflalOwnId, PflalPower};
pub use pflam::{Pflam, PflamIdentity};
pub use pflau::{Pflau, PflauAlarmType, PflauGpsStatus};
pub use pflav::Pflav;
