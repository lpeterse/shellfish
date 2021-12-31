use super::session::Process;


#[derive(Debug, Clone, Copy)]
pub struct RequestFailure<T = ()>(pub T);

impl Into<Process> for RequestFailure<Process> {
    fn into(self) -> Process {
        self.0
    }
}
