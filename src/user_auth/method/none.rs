use super::*;

#[derive(Debug)]
pub struct NoneMethod;

impl AuthMethod for NoneMethod {
    const NAME: &'static str = "none";
}
