pub struct RtpBuilder<'a> {
    version: u8,
    padding: u8,
    extension: u8,
    csrc_count: u8,
    marker: u8,
    payload_type: u8,
    sequence_number: u16,
    time_stamp: u32,
    ssrc: u32,
    payload: Option<&'a [u8]>,
}

impl<'a> RtpBuilder<'a> {
    pub fn new() -> Self {
        RtpBuilder {
            version: 0,
            padding: 0,
            extension: 0,
            csrc_count: 0,
            marker: 0,
            payload_type: 0xFF,
            sequence_number: 0,
            time_stamp: 0,
            ssrc: 0,
            payload: None,
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
    fn extension(mut self, extension: u8) -> Self {
        self.extension = extension;
        self
    }
    fn csrc_count(mut self, csrc_count: u8) -> Self {
        self.csrc_count = csrc_count;
        self
    }
    fn marker(mut self, marker: u8) -> Self {
        self.marker = marker;
        self
    }
    fn payload_type(mut self, payload_type: u8) -> Self {
        self.payload_type = payload_type;
        self
    }
    fn sequence_number(mut self, sequence_number: u16) -> Self {
        self.sequence_number = sequence_number;
        self
    }
    fn time_stamp(mut self, time_stamp: u32) -> Self {
        self.time_stamp = time_stamp;
        self
    }
    fn ssrc(mut self, ssrc: u32) -> Self {
        self.ssrc = ssrc;
        self
    }

    fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    fn build(&self, rtp_packet: &mut [u8]) {
        rtp_packet[0] = self.version;
        rtp_packet[0] |= self.padding << 2;
        rtp_packet[0] |= self.extension << 3;
        rtp_packet[0] |= self.csrc_count << 4;

        rtp_packet[1] = self.payload_type;

        rtp_packet[2..4].copy_from_slice(&self.sequence_number.to_be_bytes());
        rtp_packet[4..8].copy_from_slice(&self.time_stamp.to_be_bytes());
        rtp_packet[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
        if let Some(payload) = self.payload {
            rtp_packet[12..(12 + payload.len())].copy_from_slice(payload);
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
