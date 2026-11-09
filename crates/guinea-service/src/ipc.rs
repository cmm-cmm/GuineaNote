//! Opt-in local IPC. Owner-only credentials and named-pipe/socket permissions.
use crate::{
    live::{Live, UiCall},
    policy::Policy,
    *,
};
use interprocess::local_socket::ListenerOptions;
use interprocess::local_socket::tokio::{Stream, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, Clone)]
struct Descriptor {
    session: String,
    endpoint: String,
    token: String,
    root: PathBuf,
    write: bool,
    secrets: bool,
    pid: u32,
}
#[derive(Serialize)]
pub struct SessionInfo {
    pub session: String,
    pub root: PathBuf,
    pub write: bool,
    pub secret_read: bool,
    pub pid: u32,
    pub liveness: &'static str,
}
impl Descriptor {
    fn public(&self) -> SessionInfo {
        SessionInfo {
            session: self.session.clone(),
            root: self.root.clone(),
            write: self.write,
            secret_read: self.secrets,
            pid: self.pid,
            liveness: "unverified-until-connect",
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Wire {
    token: String,
    request: Request,
}
struct Job {
    request: Request,
    cancelled: Arc<AtomicBool>,
    reply: tokio::sync::oneshot::Sender<Response>,
}
pub struct Bridge {
    pub receiver: Receiver<UiCall>,
    pub live: Live,
    pub session: String,
    path: PathBuf,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}
pub fn directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("GUINEANOTE_SESSION_DIR") {
        return Ok(path.into());
    }
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/run")));
    root.map(|p| p.join("GuineaNote/sessions"))
        .ok_or_else(|| Error::new("IO_ERROR", "No private session directory is available"))
}
fn private(path: &Path, directory: bool) -> Result<()> {
    if crate::policy::special(&fs::symlink_metadata(path)?) {
        return Err(Error::new("OUT_OF_SCOPE", "Session paths cannot be links"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        // Ownership is checked against a newly created private process file,
        // avoiding unsafe libc calls and refusing foreign session directories.
        let probe = tempfile::tempfile()?;
        if fs::metadata(path)?.uid() != probe.metadata()?.uid() {
            return Err(Error::new(
                "OUT_OF_SCOPE",
                "Session path belongs to another user",
            ));
        }
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )?;
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            utilities, wrappers,
        };
        let sid = utilities::current_process_sid()?;
        let flags = if directory { "OICI" } else { "" };
        let sd: LocalBox<SecurityDescriptor> = format!("D:P(A;{flags};FA;;;{sid})").parse()?;
        wrappers::SetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner
                | SecurityInformation::Dacl
                | SecurityInformation::ProtectedDacl,
            Some(&sid),
            None,
            sd.dacl(),
            None,
        )?;
    }
    Ok(())
}
fn verify_private(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if crate::policy::special(&metadata) {
        return Err(Error::new("OUT_OF_SCOPE", "Session paths cannot be links"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let probe = tempfile::tempfile()?;
        if metadata.uid() != probe.metadata()?.uid() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::new(
                "OUT_OF_SCOPE",
                "Session path must be private to this user",
            ));
        }
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            constants::{SeObjectType, SecurityInformation},
            utilities, wrappers,
        };
        let sid = utilities::current_process_sid()?;
        let descriptor = wrappers::GetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Owner | SecurityInformation::Dacl,
        )?;
        let dacl = descriptor
            .dacl()
            .ok_or_else(|| Error::new("OUT_OF_SCOPE", "Session path has no private ACL"))?;
        if descriptor.owner() != Some(&*sid)
            || dacl.len() == 0
            || (0..dacl.len()).any(|i| dacl.get_ace(i).is_none_or(|ace| ace.sid() != Some(&*sid)))
        {
            return Err(Error::new(
                "OUT_OF_SCOPE",
                "Session path must be owned by and accessible only to this user",
            ));
        }
    }
    Ok(())
}
fn ensure_directory(dir: PathBuf) -> Result<PathBuf> {
    // Reject existing reparse points in every ancestor before creating a dir.
    for ancestor in dir.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(ancestor)
            && crate::policy::special(&meta)
        {
            return Err(Error::new(
                "OUT_OF_SCOPE",
                "Session directory ancestors cannot be links",
            ));
        }
    }
    if dir.exists() {
        verify_private(&dir)?;
    } else {
        fs::create_dir_all(&dir)?;
        private(&dir, true)?;
    }
    Ok(dir)
}
pub fn sessions() -> Result<Vec<SessionInfo>> {
    let dir = directory()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    verify_private(&dir)?;
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)?.take(200) {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json")
            && let Ok(descriptor) = load_descriptor(&path)
        {
            result.push(descriptor.public());
        }
    }
    result.sort_by(|a, b| a.session.cmp(&b.session));
    Ok(result)
}
fn load_descriptor(path: &Path) -> Result<Descriptor> {
    verify_private(path)?;
    let meta = fs::symlink_metadata(path)?;
    if crate::policy::special(&meta) || meta.len() > 16384 {
        return Err(Error::new("OUT_OF_SCOPE", "Invalid session descriptor"));
    }
    let descriptor: Descriptor = serde_json::from_slice(&fs::read(path)?)
        .map_err(|_| Error::new("INVALID_DATA", "Invalid session descriptor"))?;
    if descriptor.session.len() != 36 || descriptor.token.len() != 64 {
        return Err(Error::new("INVALID_DATA", "Invalid session credentials"));
    }
    Ok(descriptor)
}
fn endpoint_name(endpoint: &str) -> io::Result<interprocess::local_socket::Name<'_>> {
    #[cfg(windows)]
    {
        endpoint.to_ns_name::<interprocess::local_socket::GenericNamespaced>()
    }
    #[cfg(unix)]
    {
        endpoint.to_fs_name::<interprocess::local_socket::GenericFilePath>()
    }
}
impl Bridge {
    pub fn start(policy: Policy, wake: Option<Arc<dyn Fn() + Send + Sync>>) -> Result<Self> {
        Self::start_in(policy, wake, directory()?)
    }
    pub fn start_in(
        mut policy: Policy,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
        directory: PathBuf,
    ) -> Result<Self> {
        let directory = ensure_directory(directory)?;
        policy.exclude(&directory)?;
        let session = id();
        let token = format!("{}{}", id().replace('-', ""), id().replace('-', ""));
        #[cfg(windows)]
        let endpoint = format!("guineanote-{session}");
        #[cfg(unix)]
        let endpoint = directory
            .join(format!("{session}.sock"))
            .to_string_lossy()
            .into_owned();
        let options = ListenerOptions::new().name(endpoint_name(&endpoint)?);
        #[cfg(windows)]
        let options = {
            use interprocess::os::windows::{
                local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
            };
            let sid = windows_permissions::utilities::current_process_sid()?;
            let sddl = widestring::U16CString::from_str(format!("D:P(A;;GA;;;{sid})"))
                .map_err(|_| Error::new("INTERNAL", "Invalid security descriptor"))?;
            options.security_descriptor(SecurityDescriptor::deserialize(&sddl)?)
        };
        #[cfg(unix)]
        let options = {
            use interprocess::os::unix::local_socket::ListenerOptionsExt;
            options.mode(0o600)
        };
        let runtime = runtime()?;
        let listener = {
            let _enter = runtime.enter();
            options.create_tokio()?
        };
        let descriptor = Descriptor {
            session: session.clone(),
            endpoint,
            token: token.clone(),
            root: policy.root.clone(),
            write: policy.write,
            secrets: policy.secrets,
            pid: std::process::id(),
        };
        let path = directory.join(format!("{session}.json"));
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        private(&path, false)?;
        serde_json::to_writer(&mut file, &descriptor)
            .map_err(|_| Error::new("IO_ERROR", "Could not write session descriptor"))?;
        file.sync_all()?;
        let (ui_tx, receiver) = mpsc::sync_channel(64);
        let live = Live {
            session: session.clone(),
            tx: ui_tx,
            wake,
        };
        let (tx, rx) = mpsc::sync_channel::<Job>(64);
        let service_live = live.clone();
        thread::spawn(move || {
            let mut service = Service::new(policy, Some(service_live));
            while let Ok(job) = rx.recv() {
                let response = service.handle(job.request, job.cancelled);
                let _ = job.reply.send(response);
            }
        });
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let connections = Arc::new(AtomicUsize::new(0));
        thread::spawn(move || {
            runtime.block_on(async move {
            loop {
                let stream=tokio::select! { _=&mut stopped=>break, result=listener.accept()=>match result {Ok(stream)=>stream,Err(_)=>break} };
                if connections.fetch_add(1,Ordering::AcqRel)>=8 {connections.fetch_sub(1,Ordering::AcqRel);drop(stream);continue;}
                let tx=tx.clone();let token=token.clone();let connections=connections.clone();
                tokio::spawn(async move {
                    let result=serve_connection(stream,&token,tx).await;
                    #[cfg(test)] if let Err(error)=&result {eprintln!("IPC test diagnostic: {error}");}
                    drop(result);connections.fetch_sub(1,Ordering::AcqRel);
                });
            }
        })
        });
        Ok(Self {
            receiver,
            live,
            session,
            path,
            stop: Some(stop),
        })
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let _ = fs::remove_file(&self.path);
    }
}
fn response_error(id: &str, error: Error) -> Response {
    Response {
        schema_version: 1,
        request_id: id.into(),
        backend: "live".into(),
        session_id: None,
        revision: None,
        data: None,
        error: Some(error),
    }
}
fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}
struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
async fn serve_connection(
    mut stream: Stream,
    token: &str,
    tx: mpsc::SyncSender<Job>,
) -> Result<()> {
    let wire: Wire = serde_json::from_slice(
        &read_frame(&mut stream, &AtomicBool::new(false), Duration::from_secs(5)).await?,
    )
    .map_err(|_| Error::new("INVALID_ARGUMENT", "Invalid IPC request"))?;
    if wire.token.len() != token.len()
        || wire
            .token
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            != 0
    {
        return Err(Error::new("OUT_OF_SCOPE", "Invalid session credential"));
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let _guard = Cancel(cancelled.clone());
    let (reply, rx) = tokio::sync::oneshot::channel();
    let request_id = wire.request.request_id.clone();
    if tx
        .try_send(Job {
            request: wire.request,
            cancelled,
            reply,
        })
        .is_err()
    {
        return write_frame(
            &mut stream,
            &response_error(
                &request_id,
                Error::new("BUSY", "Agent request queue is full"),
            ),
        )
        .await;
    }
    let mut byte = [0u8; 1];
    tokio::select! {
        response=rx=>write_frame(&mut stream,&response.map_err(|_|Error::new("SESSION_GONE","Agent service stopped"))?).await,
        _=stream.read(&mut byte)=>Ok(())
    }
}
async fn read_frame(
    stream: &mut Stream,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let started = Instant::now();
    let mut chunk = [0u8; 8192];
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::new("CANCELLED", "Request cancelled"));
        }
        if started.elapsed() > timeout {
            return Err(Error::new(
                "SESSION_GONE",
                "IPC response timed out; query transaction status before retrying writes",
            ));
        }
        let n = tokio::select! { _=tokio::time::sleep(Duration::from_millis(5))=>continue, n=stream.read(&mut chunk)=>n? };
        if n == 0 {
            return Err(Error::new("SESSION_GONE", "Editor connection closed"));
        }
        buffer.extend_from_slice(&chunk[..n]);
        if buffer.len() > MAX_MESSAGE {
            return Err(Error::new("LIMIT_EXCEEDED", "IPC frame exceeds 2 MiB"));
        }
        if let Some(end) = buffer.iter().position(|b| *b == b'\n') {
            if end + 1 != buffer.len() {
                return Err(Error::new(
                    "INVALID_ARGUMENT",
                    "Only one frame is allowed per connection",
                ));
            }
            buffer.truncate(end);
            return Ok(buffer);
        }
    }
}
async fn write_frame(stream: &mut Stream, value: &impl Serialize) -> Result<()> {
    let mut data = serde_json::to_vec(value)
        .map_err(|_| Error::new("INTERNAL", "Response serialization failed"))?;
    if data.len() > MAX_MESSAGE {
        return Err(Error::new(
            "LIMIT_EXCEEDED",
            "Response exceeds frame budget",
        ));
    }
    data.push(b'\n');
    tokio::time::timeout(Duration::from_secs(5), stream.write_all(&data))
        .await
        .map_err(|_| Error::new("BUSY", "IPC peer is not reading"))??;
    Ok(())
}
#[derive(Clone)]
pub struct Client {
    descriptor: Descriptor,
}
impl Client {
    pub fn connect(session: &str) -> Result<Self> {
        if uuid::Uuid::parse_str(session).is_err() {
            return Err(Error::new("INVALID_ARGUMENT", "Invalid session ID"));
        }
        let dir = directory()?;
        verify_private(&dir)?;
        let descriptor = load_descriptor(&dir.join(format!("{session}.json")))?;
        if descriptor.session != session {
            return Err(Error::new("OUT_OF_SCOPE", "Session descriptor mismatch"));
        }
        Ok(Self { descriptor })
    }
    pub fn info(&self) -> SessionInfo {
        self.descriptor.public()
    }
    pub fn call(&self, request: Request, cancelled: &AtomicBool) -> Result<Response> {
        let request_id = request.request_id.clone();
        runtime()?.block_on(async {
            let mut stream = Stream::connect(endpoint_name(&self.descriptor.endpoint)?)
                .await
                .map_err(|_| Error::new("SESSION_GONE", "Editor session is not reachable"))?;
            write_frame(
                &mut stream,
                &Wire {
                    token: self.descriptor.token.clone(),
                    request,
                },
            )
            .await?;
            let response: Response = serde_json::from_slice(
                &read_frame(&mut stream, cancelled, Duration::from_secs(60)).await?,
            )
            .map_err(|_| Error::new("INVALID_DATA", "Invalid editor response"))?;
            if response.request_id != request_id {
                return Err(Error::new("INVALID_DATA", "IPC request ID mismatch"));
            }
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_local_transport_permissions_credentials_and_cleanup() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("public.txt"), "hello").unwrap();
        let bridge = Bridge::start_in(
            Policy::new(root.path(), false, false).unwrap(),
            None,
            root.path().join("sessions"),
        )
        .unwrap();
        let descriptor = load_descriptor(&bridge.path).unwrap();
        let client = Client {
            descriptor: descriptor.clone(),
        };
        let response = client
            .call(
                Request::new(Operation::ReadDocument {
                    document: None,
                    path: Some("public.txt".into()),
                    start: 0,
                    limit: 20,
                    expected_hash: None,
                }),
                &AtomicBool::new(false),
            )
            .unwrap()
            .result()
            .unwrap();
        assert_eq!(response["text"], "hello");
        let denied = client
            .call(
                Request::new(Operation::ReadDocument {
                    document: None,
                    path: Some(format!("sessions/{}.json", bridge.session)),
                    start: 0,
                    limit: 20,
                    expected_hash: None,
                }),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(denied.error.unwrap().code, "OUT_OF_SCOPE");
        let mut invalid = descriptor;
        invalid.token = "0".repeat(64);
        assert!(
            Client {
                descriptor: invalid
            }
            .call(
                Request::new(Operation::GetCapabilities {}),
                &AtomicBool::new(false)
            )
            .is_err()
        );
        let path = bridge.path.clone();
        drop(bridge);
        assert!(!path.exists());
    }
}
