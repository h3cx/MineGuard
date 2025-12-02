use std::path::PathBuf;

use crate::config::MinecraftVersion;

pub struct InstanceData {
    pub root_dir: PathBuf,
    pub jar_path: PathBuf,
    pub version: MinecraftVersion,
}
