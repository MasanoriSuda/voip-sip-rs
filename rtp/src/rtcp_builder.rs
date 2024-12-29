pub struct RtpBuilder<'a> {
    version: u8,
    padding: u8,
    rc: u8,
    pt: u8,
    length:u16,

impl<'a> RtpBuilder<'a> {
    pub fn new() -> Self {
        RtpBuilder {
            version: 0,
            padding: 0,
            rc: 0,
            pt: 0,
            length:0,
        }
    }
    fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }
    fn padding(mut self, padding: u8) -> Self {
        self.padding = padding;
        self
    }
    fn rc(mut self, marker: u8) -> Self {
        self.marker = marker;
        self
    }
    fn pt(mut self, payload_type: u8) -> Self {
        self.payload_type = payload_type;
        self
    }
    fn length(mut self, sequence_number: u16) -> Self {
        self.sequence_number = sequence_number;
        self
    }


    fn build(&self, rtp_packet: &mut [u8]) {
        rtp_packet[0] = self.version;
        rtp_packet[0] |= self.padding << 2;
        rtp_packet[0] |= self.rc << 3;
        rtp_packet[1] = self.pt;
        rtp_packet[2..4].copy_from_slice(&length.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_all() {
        let payload = [1u8; 4];
        let mut hoge = [0u8; 16];
        let packet: () = RtpBuilder::new()
            .payload_type(1)
            .payload(&payload)
            .build(hoge.as_mut_slice());
        println!("{:?}", hoge);
    }
}
