
pub fn assume(x: bool) -> Option<()> {
    if x {
        Some(())
    } else {
        None
    }
}