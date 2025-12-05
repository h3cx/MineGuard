use std::path::PathBuf;

use crate::config::{MinecraftType, MinecraftVersion};

#[derive(Debug, Clone)]
pub struct InstanceData {
    pub root_dir: PathBuf,
    pub jar_path: PathBuf,
    pub mc_version: MinecraftVersion,
    pub mc_type: MinecraftType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Crashed,
    Killing,
    Killed,
}
