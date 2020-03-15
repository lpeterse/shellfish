
pub struct Signal(String);

impl Signal {
    pub fn int() -> Self {
        Self(String::from("INT"))
    }
}
