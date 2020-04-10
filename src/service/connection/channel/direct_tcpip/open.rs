use crate::codec::*;

#[derive(Debug, Clone)]
pub struct DirectTcpIpOpen {
    pub(crate) dst_host: String,
    pub(crate) dst_port: u32,
    pub(crate) src_addr: String,
    pub(crate) src_port: u32,
}

impl Encode for DirectTcpIpOpen {
    fn size(&self) -> usize {
        self.dst_host.size() + 4 + self.src_addr.size() + 4
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_encode(&self.dst_host);
        e.push_u32be(self.dst_port);
        e.push_encode(&self.src_addr);
        e.push_u32be(self.src_port);
    }
}

impl Decode for DirectTcpIpOpen {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self {
            dst_host: d.take_decode()?,
            dst_port: d.take_u32be()?,
            src_addr: d.take_decode()?,
            src_port: d.take_u32be()?,
        }
        .into()
    }
}
