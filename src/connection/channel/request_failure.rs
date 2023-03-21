use super::session::Process;


#[derive(Debug, Clone, Copy)]
pub struct RequestFailure<T = ()>(pub T);

impl Into<Box<dyn Process>> for RequestFailure<Box<dyn Process>> {
    fn into(self) -> Box<dyn Process> {
        self.0
    }
}
