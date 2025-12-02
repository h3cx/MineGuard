use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::error::VersionError;

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

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('.');

        let major_str = split.next().ok_or(VersionError::MissingMajor)?;
        let minor_str = split.next().ok_or(VersionError::MissingMinor)?;
        let patch_str = split.next().ok_or(VersionError::MissingPatch)?;

        if split.next().is_some() {
            return Err(VersionError::ExtraComponents);
        }

        let major = major_str
            .parse::<u32>()
            .map_err(|_| VersionError::IncorrectMajor(major_str.to_string()))?;

        let minor = minor_str
            .parse::<u32>()
            .map_err(|_| VersionError::IncorrectMinor(minor_str.to_string()))?;

        let patch = patch_str
            .parse::<u32>()
            .map_err(|_| VersionError::IncorrectPatch(patch_str.to_string()))?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl FromStr for Snapshot {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (year_str, rest) = s
            .split_once('w')
            .ok_or(VersionError::InvalidSnapshotFormat)?;

        if rest.len() < 3 {
            return Err(VersionError::InvalidSnapshotFormat);
        }

        let week_str = &rest[..2];
        let build_str = &rest[2..];

        let year = year_str
            .parse::<u32>()
            .map_err(|_| VersionError::IncorrectYear(year_str.to_string()))?;

        let week = week_str
            .parse::<u32>()
            .map_err(|_| VersionError::IncorrectWeek(week_str.to_string()))?;

        let build = if build_str.len() == 1 {
            build_str.chars().next().unwrap()
        } else {
            return Err(VersionError::IncorrectBuild(build_str.to_string()));
        };

        Ok(Self { year, week, build })
    }
}

impl FromStr for MinecraftVersion {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(ver) = Version::from_str(s) {
            return Ok(MinecraftVersion::Release(ver));
        }

        if let Ok(snap) = Snapshot::from_str(s) {
            return Ok(MinecraftVersion::Snapshot(snap));
        }

        Err(VersionError::UnknownVersionFormat(s.to_string()))
    }
}
