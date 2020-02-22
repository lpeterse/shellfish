/// All types representing SSH_MSG_* messages shall implement this trait.
pub trait Message {
    // The message number as speicified in the RFCs.
    const NUMBER: u8;
}
