struct RtpPacketParser<'a> {
    rtp_packet: &'a [u8],
}

impl<'a> RtpPacketParser<'a> {
    pub fn new(rtp_packet: &'a [u8]) -> Self {
        let retval = RtpPacketParser {
            rtp_packet: &rtp_packet,
        };

        return retval;
    }

    fn version(&self) -> u8 {
        return self.rtp_packet[0] & 0b0000_0011;
    }

    fn padding(&self) -> u8 {
        return self.rtp_packet[0] & 0b0000_0100;
    }

    fn extension(&self) -> u8 {
        return self.rtp_packet[0] & 0b0000_1000;
    }

    fn csrc_count(&self) -> u8 {
        return self.rtp_packet[0] & 0b1111_0000;
    }
    fn payload_type(&self) -> u8 {
        return self.rtp_packet[1];
    }
    fn sequence_number(&self) -> u16 {
        return u16::from_be_bytes(self.rtp_packet[2..4].try_into().unwrap());
    }

    fn time_stamp(&self) -> u32 {
        return u32::from_be_bytes(self.rtp_packet[4..8].try_into().unwrap());
    }
    fn ssrc(&self) -> u32 {
        return u32::from_be_bytes(self.rtp_packet[8..12].try_into().unwrap());
    }

    fn payload(&self) -> &'a [u8] {
        return &self.rtp_packet[12..self.rtp_packet.len()];
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_all() {
        let hoge = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1];
        let packet = RtpPacketParser::new(&hoge);

        println!("{:?}", packet.ssrc());
        println!("{:?}", packet.payload_type());
    }
}
