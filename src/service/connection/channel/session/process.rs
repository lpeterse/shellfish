use crate::pipe::*;

pub struct Process<T> (T);

impl <T> Process<T> {
    pub (super) fn new(t: T) -> Self {
        Self (t)
    }
}