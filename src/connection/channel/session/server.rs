use super::*;

#[derive(Debug)]
pub struct SessionServer(pub Arc<Mutex<SessionServerState>>);

pub struct SessionHandle;

pub trait SessionHandler {
    fn on_env_request(&mut self, req: EnvRequest);
    fn on_pty_request(&mut self, req: PtyRequest);
    fn on_shell_request(self: Box<Self>, req: ShellRequest);
    fn on_exec_request(self: Box<Self>, req: ExecRequest);
    fn on_subsystem_request(self: Box<Self>, req: SubsystemRequest);
}

#[derive(Debug)]
pub struct SessionRequest {
    pub chan: SessionServer,
    pub resp: oneshot::Sender<Result<(), OpenFailure>>,
}

impl SessionRequest {
    fn accept(self, handler: Box<dyn SessionHandler>) {
        panic!()
    }
    fn reject(self) {
        panic!()
    }
}

pub struct EnvRequest;

impl EnvRequest {
    fn accept(self) {
        panic!()
    }
    fn reject(self) {
        drop(self)
    }
}

pub struct PtyRequest;

impl PtyRequest {
    fn accept(self) {
        panic!()
    }
    fn reject(self) {
        drop(self)
    }
}

pub struct ShellRequest;

impl ShellRequest {
    fn accept(self, proc: Process) -> SessionHandle {
        panic!()
    }
    fn reject(self) {
        drop(self)
    }
}

pub struct ExecRequest {
    command: String,
}

impl ExecRequest {
    fn command(&self) -> &str {
        &self.command
    }
    fn accept(self, proc: Process) -> SessionHandle {
        panic!()
    }
    fn reject(self) {
        drop(self)
    }
}

pub struct SubsystemRequest {
    subsystem: String,
}

impl SubsystemRequest {
    fn subsystem(&self) -> &str {
        &self.subsystem
    }
    fn accept(self, proc: Process) -> SessionHandle {
        panic!()
    }
    fn reject(self) {
        drop(self)
    }
}
