use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinecraftType {
    Vanilla,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub year: u32,
    pub week: u32,
    pub build: char,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinecraftVersion {
    Release(Version),
    Snapshot(Snapshot),
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}w{:02}{}", self.year, self.week, self.build)
    }
}
