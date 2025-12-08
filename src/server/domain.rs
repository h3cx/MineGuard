use std::{ops::RangeInclusive, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    fs::{File, create_dir, read, read_dir},
    io::{self, AsyncWriteExt},
    sync::{RwLock, watch},
};
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    config::{self, MinecraftType, MinecraftVersion, StreamSource, Version, stream::InstanceEvent},
    error::{CreationError, ServerError, SubscribeError},
    instance::InstanceHandle,
    manifests::vanilla::{VanillaManifestV2, VanillaManifestV2Version, VanillaReleaseManifest},
    server,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MineGuardConfig {
    uuid: Uuid,
    pub server_dir: PathBuf,
    pub jar_path: PathBuf,
    pub mc_version: MinecraftVersion,
    pub mc_type: MinecraftType,
}

#[derive(Debug)]
pub struct MineGuardServer {
    pub handle: RwLock<InstanceHandle>,
    pub config: RwLock<MineGuardConfig>,
}

impl MineGuardConfig {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            server_dir: PathBuf::new(),
            jar_path: PathBuf::new(),
            mc_version: MinecraftVersion::Release(Version::from_str("0.00.00").unwrap()),
            mc_type: MinecraftType::Vanilla,
        }
    }
}

impl MineGuardServer {
    async fn load_cfg_handle(
        config: MineGuardConfig,
        handle: InstanceHandle,
    ) -> Result<Self, CreationError> {
        Ok(Self {
            config: RwLock::new(config),
            handle: RwLock::new(handle),
        })
    }
    pub async fn create(
        mc_version: MinecraftVersion,
        mc_type: MinecraftType,
        directory: PathBuf,
    ) -> Result<Self, CreationError> {
        if !directory.is_dir() {
            return Err(CreationError::DirectoryError);
        }

        let uuid = Uuid::new_v4();

        let server_root = directory.join(uuid.to_string());
        let jar_path_rel =
            PathBuf::from_str("server.jar").map_err(|_| CreationError::DirectoryError)?;
        let jar_path_full = server_root.join(jar_path_rel.clone());

        create_dir(server_root.clone())
            .await
            .map_err(|_| CreationError::DirectoryError)?;

        let internal_dir = server_root.join(".mineguard");
        create_dir(internal_dir)
            .await
            .map_err(|_| CreationError::DirectoryError)?;

        let mut url = String::new();

        if mc_type == MinecraftType::Vanilla {
            let vanilla_manifest = VanillaManifestV2::load()
                .await
                .map_err(|_| CreationError::ManifestError)?;

            let find_ver = match vanilla_manifest
                .find(mc_version.clone())
                .map_err(|_| CreationError::ManifestError)?
            {
                Some(val) => val,
                None => return Err(CreationError::VersionError),
            };

            let release_manifest = VanillaReleaseManifest::load(find_ver)
                .await
                .map_err(|_| CreationError::ManifestError)?;

            url = release_manifest.server_url();
        }

        let resp = reqwest::get(url)
            .await
            .map_err(|_| CreationError::NetworkError)?;
        let mut body = resp
            .bytes()
            .await
            .map_err(|_| CreationError::NetworkError)?;
        let mut out = File::create(jar_path_full)
            .await
            .map_err(|_| CreationError::DirectoryError)?;
        out.write_all_buf(&mut body)
            .await
            .map_err(|_| CreationError::DirectoryError)?;

        let config = MineGuardConfig {
            uuid: uuid,
            server_dir: server_root,
            jar_path: jar_path_rel,
            mc_version: mc_version,
            mc_type: mc_type,
        };

        let handle = InstanceHandle::new_with_params(
            config.server_dir.clone(),
            config.jar_path.clone(),
            config.mc_version.clone(),
            config.mc_type.clone(),
        )
        .map_err(|_| CreationError::CreationError)?;

        let server = MineGuardServer {
            config: RwLock::new(config),
            handle: RwLock::new(handle),
        };

        Ok(server)
    }

    pub async fn start(&self) -> Result<(), ServerError> {
        let mut handle_w = self.handle.write().await;
        let res = handle_w.start().await;
        res
    }

    pub async fn kill(&self) -> Result<(), ServerError> {
        let mut handle_w = self.handle.write().await;
        let res = handle_w.kill().await;
        res
    }
    pub async fn stop(&self) -> Result<(), ServerError> {
        let mut handle_w = self.handle.write().await;
        let res = handle_w.stop().await;
        res
    }

    pub async fn subscribe(
        &self,
        stream: StreamSource,
    ) -> Result<BroadcastStream<InstanceEvent>, SubscribeError> {
        let handle_r = self.handle.read().await;
        let res = handle_r.subscribe(stream);
        res
    }

    pub async fn accept_eula(&self) -> Result<(), ServerError> {
        let config_r = self.config.read().await;
        let eula_path = config_r.server_dir.join("eula.txt");

        let mut out = File::create(eula_path)
            .await
            .map_err(|_| ServerError::NoEULA)?;

        out.write_all(b"#Generated by MineGuard\neula=true\n")
            .await
            .map_err(|_| ServerError::WriteEULAFailed)?;

        Ok(())
    }

    pub async fn write_config(&self) -> Result<(), ServerError> {
        let config_r = self.config.read().await;
        let root_path = config_r.server_dir.clone();
        let config_clone = config_r.clone();
        drop(config_r);

        let config_path = root_path.join(".mineguard/config.json");

        let json = serde_json::to_vec_pretty(&config_clone).map_err(|_| ServerError::FileIO)?;

        File::create(config_path)
            .await
            .map_err(|_| ServerError::FileIO)?
            .write_all(&json)
            .await
            .map_err(|_| ServerError::FileIO)?;

        Ok(())
    }

    pub async fn load(path: &PathBuf) -> Result<Self, CreationError> {
        let config_path = path.join(".mineguard/config.json");

        let data = read(config_path)
            .await
            .map_err(|_| CreationError::DirectoryError)?;

        let config: MineGuardConfig =
            serde_json::from_slice(&data).map_err(|_| CreationError::CreationError)?;
        let handle = InstanceHandle::new_with_config(config.clone())
            .map_err(|_| CreationError::CreationError)?;

        MineGuardServer::load_cfg_handle(config, handle).await
    }

    pub async fn load_all(path: PathBuf) -> Result<Vec<Self>, CreationError> {
        let mut dirs = Vec::new();
        let mut entries = read_dir(path)
            .await
            .map_err(|_| CreationError::DirectoryError)?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| CreationError::DirectoryError)?
        {
            let meta = entry
                .metadata()
                .await
                .map_err(|_| CreationError::DirectoryError)?;
            if meta.is_dir() {
                dirs.push(entry.path());
            }
        }

        let mut servers: Vec<Self> = Vec::new();

        for v in dirs {
            println!("{}", v.to_str().unwrap());
            servers.push(Self::load(&v).await?);
        }

        Ok(servers)
    }
}
