use crate::util::codec::{SshEncode, SshEncoder};

#[derive(Clone, Debug)]
pub struct PtySpecification;

impl SshEncode for &PtySpecification {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        todo!()
    }
}