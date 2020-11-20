mod create;

pub use create::*;
use uuid::Uuid;

pub struct Match {
    pub uuid : Uuid
}

pub struct MatchCollection {
    pub uuid: Uuid
}