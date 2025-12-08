use std::{path::PathBuf, process::Stdio, sync::Arc, time::Duration};

use chrono::Utc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::{self, Child},
    sync::{RwLock, broadcast, mpsc},
    time::sleep,
};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[cfg(feature = "events")]
use crate::config::stream::InstanceEvent;
use crate::{
    config::{
        MinecraftType, MinecraftVersion, StreamSource,
        stream::{EventPayload, InternalEvent},
    },
    error::{HandleError, ServerError, SubscribeError},
    server::domain::MineGuardConfig,
};

use super::{InstanceData, InstanceStatus};

#[derive(Debug)]
pub struct InstanceHandle {
    pub data: InstanceData,
    pub status: Arc<RwLock<InstanceStatus>>,
    stdout_tx: broadcast::Sender<InstanceEvent>,
    stderr_tx: Option<broadcast::Sender<InstanceEvent>>,
    #[cfg(feature = "events")]
    events_tx: broadcast::Sender<InstanceEvent>,
    #[cfg(feature = "events")]
    internal_events_tx: mpsc::Sender<InstanceEvent>,
    #[cfg(feature = "events")]
    internal_events_rx: Option<mpsc::Receiver<InstanceEvent>>,
    stdin_tx: mpsc::Sender<String>,
    stdin_rx: Option<mpsc::Receiver<String>>,
    child: Option<Arc<RwLock<Child>>>,
    shutdown: CancellationToken,
    internal_bus_tx: broadcast::Sender<InternalEvent>,
}

impl InstanceHandle {
    pub fn new_with_config(config: MineGuardConfig) -> Result<Self, HandleError> {
        InstanceHandle::new_with_params(
            config.server_dir,
            config.jar_path,
            config.mc_version,
            config.mc_type,
        )
    }
    pub fn new_with_params(
        root_dir: PathBuf,
        jar_path: PathBuf,
        mc_version: MinecraftVersion,
        mc_type: MinecraftType,
    ) -> Result<Self, HandleError> {
        let parsed_version: MinecraftVersion = mc_version;

        let root: PathBuf = root_dir.clone().into();
        if !root.exists() || !root.is_dir() {
            return Err(HandleError::InvalidDirectory(
                root_dir.to_str().unwrap().to_string(),
            ));
        }

        let path: PathBuf = jar_path.clone().into();
        let conc = root.join(path.clone());
        if !path.is_relative() || !conc.is_file() {
            return Err(HandleError::InvalidPathJAR(
                jar_path.to_str().unwrap().to_string(),
            ));
        }

        let data = InstanceData {
            root_dir: root,
            jar_path: path,
            mc_version: parsed_version,
            mc_type,
        };

        let status = InstanceStatus::Stopped;

        let (stdin_tx, stdin_rx) = mpsc::channel(1024);
        let (internal_tx, internal_rx) = mpsc::channel(1024);
        Ok(Self {
            data,
            status: Arc::new(RwLock::new(status)),
            stdout_tx: broadcast::Sender::new(2048),
            stderr_tx: None,
            #[cfg(feature = "events")]
            events_tx: broadcast::Sender::new(2048),
            #[cfg(feature = "events")]
            internal_events_tx: internal_tx,
            #[cfg(feature = "events")]
            internal_events_rx: Some(internal_rx),
            stdin_tx,
            stdin_rx: Some(stdin_rx),
            child: None,
            shutdown: CancellationToken::new(),
            internal_bus_tx: broadcast::Sender::new(2048),
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
        self.setup_loopback()?;

        self.transition_status(InstanceStatus::Starting).await;

        let command = self.build_start_command();
        let child = self.spawn_child_process(command)?;

        self.setup_stream_pumps(child)?;

        self.setup_parser()?;

        let mut rx = self.internal_bus_tx.subscribe();

        loop {
            match rx.recv().await {
                Ok(event) => {
                    if event == InternalEvent::ServerStarted {
                        self.transition_status(InstanceStatus::Running).await;
                        break;
                    }
                    continue;
                }
                _ => continue,
            }
        }

        Ok(())
    }

    async fn validate_start_parameters(&self) -> Result<(), ServerError> {
        if self.child.is_some() {
            return Err(ServerError::AlreadyRunning);
        }

        Ok(())
    }

    async fn transition_status(&self, status: InstanceStatus) {
        let r_guard = self.status.read().await;
        let old = r_guard.clone();
        drop(r_guard);

        let new = status.clone();

        let mut guard = self.status.write().await;
        *guard = status;
        drop(guard);

        let event = InstanceEvent {
            id: Uuid::new_v4(),

            timestamp: Utc::now(),

            payload: EventPayload::StateChange { old, new },
        };

        _ = self.internal_events_tx.send(event).await;
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

        let stdout_status = self.status.clone();
        let stderr_status = self.status.clone();
        let internal_tx1 = self.internal_events_tx.clone();
        let internal_tx2 = self.internal_events_tx.clone();

        tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout).lines();
            loop {
                match stdout_reader.next_line().await {
                    Ok(Some(line)) => {
                        let _ = stdout_tx.send(InstanceEvent::stdout(line));
                    }
                    _ => {
                        let status_guard = stdout_status.read().await;
                        let state = status_guard.clone();
                        if state == InstanceStatus::Running && state == InstanceStatus::Starting {
                            let old = status_guard.clone();
                            drop(status_guard);
                            let mut status = stdout_status.write().await;
                            *status = InstanceStatus::Crashed;
                            let event = InstanceEvent {
                                id: Uuid::new_v4(),

                                timestamp: Utc::now(),

                                payload: EventPayload::StateChange {
                                    old,
                                    new: status.clone(),
                                },
                            };

                            _ = internal_tx1.send(event).await;
                            drop(status);
                            break;
                        }
                        drop(status_guard);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            loop {
                match stderr_reader.next_line().await {
                    Ok(Some(line)) => {
                        let _ = stderr_tx.send(InstanceEvent::stderr(line));
                    }
                    _ => {
                        let status_guard = stderr_status.read().await;
                        let state = status_guard.clone();
                        if state == InstanceStatus::Running && state == InstanceStatus::Starting {
                            let old = status_guard.clone();
                            drop(status_guard);
                            let mut status = stderr_status.write().await;
                            *status = InstanceStatus::Crashed;
                            let event = InstanceEvent {
                                id: Uuid::new_v4(),

                                timestamp: Utc::now(),

                                payload: EventPayload::StateChange {
                                    old,
                                    new: status.clone(),
                                },
                            };

                            _ = internal_tx2.send(event).await;
                            drop(status);
                            break;
                        }
                        drop(status_guard);
                    }
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
                        if let Some(cmd) = maybe_cmd {
                            _ = writer.write_all(cmd.as_bytes()).await;
                            _ = writer.flush().await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    #[cfg(all(feature = "events", any(feature = "mc-vanilla")))]
    fn setup_loopback(&mut self) -> Result<(), ServerError> {
        let shutdown1 = self.shutdown.clone();

        let event_tx1 = self.events_tx.clone();
        //internal mpsc to broadcast loopback
        if let Some(mut internal_rx) = self.internal_events_rx.take() {
            tokio::spawn(async move {
                let tx = event_tx1;
                loop {
                    tokio::select! {
                        _ = shutdown1.cancelled() => {
                            break;
                        }

                        maybe_event = internal_rx.recv() => {
                            if let Some(event) = maybe_event {
                                _ = tx.send(event);
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }

    #[cfg(all(feature = "events", any(feature = "mc-vanilla")))]
    fn setup_parser(&mut self) -> Result<(), ServerError> {
        use crate::config::LogMeta;

        let stdout_stream = self
            .subscribe(StreamSource::Stdout)
            .map_err(|_| ServerError::NoStdoutPipe)?;
        let shutdown2 = self.shutdown.clone();
        let bus_tx = self.internal_bus_tx.clone();

        #[cfg(feature = "mc-vanilla")]
        if self.data.mc_type == MinecraftType::Vanilla {
            tokio::spawn(async move {
                let mut rx = stdout_stream;
                let tx = bus_tx;

                loop {
                    tokio::select! {
                        _ = shutdown2.cancelled() => {
                            break;
                        }
                        next_line = rx.next() => {
                            if let Some(Ok(val)) = next_line {
                                let event_line = match val.payload {
                                    EventPayload::StdLine{line} => {
                                        line
                                    },
                                    _ => continue,
                                };

                                let meta = match LogMeta::new(event_line.line) {
                                    Ok(Some(log_meta)) => {
                                        log_meta
                                    },
                                    _ => continue,
                                };

                                match meta.parse_event() {
                                    Ok(Some(event)) => _ = tx.send(event),
                                    _ => continue,
                                }
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }

    pub async fn kill(&mut self) -> Result<(), ServerError> {
        if let Some(child_arc) = self.child.clone() {
            self.transition_status(InstanceStatus::Killing).await;
            let mut child = child_arc.write().await;

            child.kill().await.map_err(|_| ServerError::CommandFailed)?;

            self.transition_status(InstanceStatus::Killed).await;
            sleep(Duration::from_secs(1)).await;
            self.shutdown.cancel();
            self.child = None;
            Ok(())
        } else {
            Err(ServerError::NotRunning)
        }
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        if let Some(child_arc) = self.child.clone() {
            self.transition_status(InstanceStatus::Stopping).await;

            _ = self.send_command("stop").await;
            let mut child = child_arc.write().await;
            child.wait().await.map_err(|_| ServerError::CommandFailed)?;

            self.transition_status(InstanceStatus::Stopped).await;
            sleep(Duration::from_secs(1)).await;
            self.shutdown.cancel();
            self.child = None;
            Ok(())
        } else {
            Err(ServerError::NotRunning)
        }
    }

    pub fn subscribe(
        &self,
        stream: StreamSource,
    ) -> Result<BroadcastStream<InstanceEvent>, SubscribeError> {
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
            #[cfg(feature = "events")]
            StreamSource::Event => {
                let rx = self.events_tx.subscribe();
                Ok(BroadcastStream::new(rx))
            }
        }
    }
}
