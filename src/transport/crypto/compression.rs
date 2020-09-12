pub trait CompressionAlgorithm {
    const NAME: &'static str; 
}

pub struct NoCompression {}

impl CompressionAlgorithm for NoCompression {
    const NAME: &'static str = "none";
}
