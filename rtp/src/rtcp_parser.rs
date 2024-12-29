struct RtcpPacketParser<'a> {
    rtp_packet: &'a [u8],
}

impl<'a> RtcpPacketParser<'a> {
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

    fn rc(&self) -> u8 {
        return self.rtp_packet[0] & 0b1111_1000;
    }
    fn pt(&self) -> u8 {
        return self.rtp_packet[1];
    }
    fn sequence_number(&self) -> u16 {
        return u16::from_be_bytes(self.rtp_packet[2..4].try_into().unwrap());
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
