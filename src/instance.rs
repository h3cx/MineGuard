use std::{path::PathBuf, process::Stdio, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{self, Child},
    sync::{RwLock, broadcast, mpsc},
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;

use crate::{
    config::{MinecraftType, MinecraftVersion, StreamLine, StreamSource},
    error::{HandleError, ServerError, SubscribeError},
};

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

#[derive(Debug)]
pub struct InstanceHandle {
    pub data: InstanceData,
    pub status: Arc<RwLock<InstanceStatus>>,
    stdout_tx: broadcast::Sender<StreamLine>,
    stderr_tx: Option<broadcast::Sender<StreamLine>>,
    stdin_tx: mpsc::Sender<String>,
    stdin_rx: Option<mpsc::Receiver<String>>,
    child: Option<Arc<RwLock<Child>>>,
    shutdown: CancellationToken,
}

impl InstanceHandle {
    pub fn new_with_params(
        root_dir: &str,
        jar_path: &str,
        mc_version: &str,
        mc_type: MinecraftType,
    ) -> Result<Self, HandleError> {
        let parsed_version: MinecraftVersion = mc_version
            .parse()
            .map_err(|_| HandleError::InvalidVersion(mc_version.to_string()))?;

        let root: PathBuf = root_dir.into();
        if !root.exists() || !root.is_dir() {
            return Err(HandleError::InvalidDirectory(root_dir.to_string()));
        }

        let path: PathBuf = jar_path.into();
        let conc = root.join(path.clone());
        if !path.is_relative() || !conc.is_file() {
            return Err(HandleError::InvalidPathJAR(jar_path.to_string()));
        }

        let data = InstanceData {
            root_dir: root,
            jar_path: path,
            mc_version: parsed_version,
            mc_type: mc_type,
        };

        let status = InstanceStatus::Stopped;

        let (stdin_tx, stdin_rx) = mpsc::channel(1024);
        Ok(Self {
            data,
            status: Arc::new(RwLock::new(status)),
            stdout_tx: broadcast::Sender::new(2048),
            stderr_tx: None,
            stdin_tx,
            stdin_rx: Some(stdin_rx),
            child: None,
            shutdown: CancellationToken::new(),
        })
    }

    pub async fn send_command<S: Into<String>>(&self, cmd: S) -> Result<(), ServerError> {
        let mut command = cmd.into();
        if !command.ends_with('\n') {
            command.push('\n');
        }

        self.stdin_tx
            .send(command)
            .await
            .map_err(|_| ServerError::StdinWriteFailed)?;

        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.validate_start_parameters().await?;

        self.transition_status(InstanceStatus::Starting).await;

        let command = self.build_start_command();
        let child = self.spawn_child_process(command)?;

        self.setup_stream_pumps(child)?;

        self.transition_status(InstanceStatus::Running).await;

        Ok(())
    }

    async fn validate_start_parameters(&self) -> Result<(), ServerError> {
        if self.child.is_some() {
            return Err(ServerError::AlreadyRunning);
        }

        Ok(())
    }

    async fn transition_status(&self, status: InstanceStatus) {
        let mut guard = self.status.write().await;
        *guard = status;
    }

    fn build_start_command(&self) -> process::Command {
        let mut command = process::Command::new("java");
        command
            .arg("-jar")
            .arg(&self.data.jar_path)
            .arg("nogui")
            .current_dir(&self.data.root_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped());

        command.process_group(0);
        command
    }

    fn spawn_child_process(&self, mut command: process::Command) -> Result<Child, ServerError> {
        command.spawn().map_err(|_| ServerError::CommandFailed)
    }

    fn setup_stream_pumps(&mut self, mut child: Child) -> Result<(), ServerError> {
        let stdout = child.stdout.take().ok_or(ServerError::NoStdoutPipe)?;
        let stderr = child.stderr.take().ok_or(ServerError::NoStderrPipe)?;
        let stdin = child.stdin.take().ok_or(ServerError::NoStdinPipe)?;

        let child = Arc::new(RwLock::new(child));
        self.child = Some(child);

        let stdout_tx = self.stdout_tx.clone();
        let stderr_tx = broadcast::Sender::new(2048);
        self.stderr_tx = Some(stderr_tx.clone());
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout).lines();
            loop {
                match stdout_reader.next_line().await {
                    Ok(Some(line)) => {
                        let _ = stdout_tx.send(StreamLine::stdout(line));
                    }
                    Ok(None) => {
                        break;
                    }
                    _ => break,
                }
            }
        });

        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            loop {
                match stderr_reader.next_line().await {
                    Ok(Some(line)) => {
                        let _ = stderr_tx.send(StreamLine::stderr(line));
                    }
                    Ok(None) => {
                        break;
                    }
                    _ => break,
                }
            }
        });

        let mut stdin_rx = self.stdin_rx.take().ok_or(ServerError::NoStdinPipe)?;

        tokio::spawn(async move {
            let mut writer = BufWriter::new(stdin);

            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        break;
                    }
                    maybe_cmd = stdin_rx.recv() => {
                        match maybe_cmd {
                            Some(cmd) => {
                                _ = writer.write_all(cmd.as_bytes()).await;
                                _ = writer.flush().await;
                            },
                            _ => (),
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn kill(&mut self) -> Result<(), ServerError> {
        if let Some(child_arc) = self.child.clone() {
            let mut status = self.status.write().await;
            *status = InstanceStatus::Killing;
            let mut child = child_arc.write().await;

            child.kill().await.map_err(|_| ServerError::CommandFailed)?;

            self.shutdown.cancel();
            self.child = None;

            let mut status = self.status.write().await;
            *status = InstanceStatus::Killed;
            Ok(())
        } else {
            Err(ServerError::NotRunning)
        }
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        if let Some(child_arc) = self.child.clone() {
            let mut status = self.status.write().await;
            *status = InstanceStatus::Stopping;

            _ = self.send_command("stop").await;

            let mut child = child_arc.write().await;
            child.wait().await.map_err(|_| ServerError::CommandFailed)?;
            self.shutdown.cancel();
            self.child = None;
            let mut status = self.status.write().await;
            *status = InstanceStatus::Stopped;
            Ok(())
        } else {
            Err(ServerError::NotRunning)
        }
    }

    pub fn subscribe(
        &self,
        stream: StreamSource,
    ) -> Result<BroadcastStream<StreamLine>, SubscribeError> {
        match stream {
            StreamSource::Stdout => {
                let rx = self.stdout_tx.subscribe();
                Ok(BroadcastStream::new(rx))
            }
            StreamSource::Stderr => {
                let rx = match &self.stderr_tx {
                    Some(value) => value.subscribe(),
                    None => return Err(SubscribeError::NoStderr),
                };
                Ok(BroadcastStream::new(rx))
            }
        }
    }
}
