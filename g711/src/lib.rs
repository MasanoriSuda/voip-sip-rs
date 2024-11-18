fn alaw_compress(lseg: i32, linbuf: &mut [i16], logbuf: &mut [i16]) {
    let mut ix: i16;
    let mut iexp: i16;

    for n in 0..(lseg as usize) {
        ix = if linbuf[n] < 0 {
            (!linbuf[n]) >> 4
        } else {
            (linbuf[n]) >> 4
        };

        if ix > 15 {
            iexp = 1;
            while ix > 16 + 15 {
                ix >>= 1;
                iexp += 1;
            }
            ix -= 16;

            ix += iexp << 4;
        }
        if linbuf[n as usize] >= 0 {
            ix |= 0x0080;
        }

        logbuf[n as usize] = ix ^ (0x0055);
    }
}

fn alaw_expand(lseg: i32, logbuf: &mut [i16], linbuf: &mut [i16]) {
    let mut ix: i16;
    let mut mant: i16;
    let mut iexp: i16;

    for n in 0..lseg {
        ix = logbuf[n as usize] ^ (0x0055);

        ix &= 0x007F;
        iexp = ix >> 4;
        mant = ix & (0x000F);
        if iexp > 0 {
            mant = mant + 16;
        }

        mant = (mant << 4) + (0x0008);

        if iexp > 1 {
            mant = mant << (iexp - 1);
        }

        linbuf[n as usize] = if logbuf[n as usize] > 127 {
            mant
        } else {
            !mant
        }
    }
}

fn ulaw_compress(lseg: i32, linbuf: &mut [i16], logbuf: &mut [i16]) {
    let mut i: i16;
    let mut absno: i16;
    let mut segno: i16;
    let mut low_nibble: i16;
    let mut high_nibble: i16;

    for n in 0..lseg {
        absno = if linbuf[n as usize] < 0 {
            ((!linbuf[n as usize]) >> 2) + 33
        } else {
            (linbuf[n as usize] >> 2) + 33
        };

        if absno > 0x1FFF {
            absno = 0x1FFF;
        }
        i = absno >> 6;
        segno = 1;
        while i != 0 {
            segno += 1;
            i >>= 1;
        }

        high_nibble = 0x0008 - segno;

        low_nibble = (absno >> segno) & 0x000F;
        low_nibble = 0x000F - low_nibble;

        logbuf[n as usize] = (high_nibble << 4) | low_nibble;

        if linbuf[n as usize] >= 0 {
            logbuf[n as usize] = logbuf[n as usize] | 0x0080;
        }
    }
}

fn ulaw_expand(lseg: u32, logbuf: &mut [i16], linbuf: &mut [i16]) {
    let mut segment: i16;
    let mut mantissa: i16;
    let mut exponent: i16;
    let mut sign: i16;
    let mut step: i16;

    for n in 0..lseg {
        sign = if logbuf[n as usize] > 0x0080 { -1 } else { 1 };
        mantissa = !logbuf[n as usize];
        exponent = (mantissa >> 4) & 0x0007;
        segment = exponent + 1;
        mantissa = mantissa & 0x000F;

        step = 4 << segment;

        linbuf[n as usize] = sign * ((0x0080 << exponent) + step * mantissa + step / 2 - 4 * 33)
    }
}
