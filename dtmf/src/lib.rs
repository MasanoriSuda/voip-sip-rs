const MAX_DTMF_DIGITS: u32 = 128;
const DTMF_MATRIX_SIZE: usize = 4;

const DTMF_THRESHOLD: f32 = 8.0e7;

const DEF_DTMF_NORMAL_TWIST: f32 = 6.31; /* 8.0dB */
const DEF_DTMF_REVERSE_TWIST: f32 = 2.51; /* 4.01dB */

const DTMF_RELATIVE_PEAK_ROW: f32 = 6.3; /* 8dB */
const DTMF_RELATIVE_PEAK_COL: f32 = 6.3; /* 8dB */
const DTMF_TO_TOTAL_ENERGY: f32 = 42.0;

const DEFAULT_SAMPLE_RATE: u32 = 8000;

/* How many successive hits needed to consider begin of a digit
 * IE. Override with dtmf_hits_to_begin=4 in dsp.conf
 */
const DEF_DTMF_HITS_TO_BEGIN: i32 = 2;

/* How many successive misses needed to consider end of a digit
 * IE. Override with dtmf_misses_to_end=4 in dsp.conf
 */
const DEF_DTMF_MISSES_TO_END: i32 = 3; /* How many successive misses needed to consider end of a digit */

struct GoertzelResult {
    value: i32,
    power: i32,
}

#[derive(Copy, Clone, Default)]
struct GoertzelState {
    /* The previous previous sample calculation (No binary point just plain int) */
    v2: i32,
    /* The previous sample calculation (No binary point just plain int) */
    v3: i32,
    /* v2 and v3 power of two exponent to keep value in int range */
    chunky: i32,
    /* 15 bit fixed point goertzel coefficient = 2 * cos(2 * pi * freq / sample_rate) */
    fac: i32,
}

impl GoertzelState {
    fn new(freq: f32, sample_rate: f32) -> Self {
        GoertzelState {
            v2: 0,
            v3: 0,
            chunky: 0,
            fac: (32768.0 * 2.0 * (2.0 * std::f32::consts::PI * freq / sample_rate).cos()) as i32,
        }
    }
}

#[derive(Default)]
struct DtmfDetectState {
    row_out: [GoertzelState; DTMF_MATRIX_SIZE],
    col_out: [GoertzelState; DTMF_MATRIX_SIZE],
    hits: i32,   /* How many successive hits we have seen already */
    misses: i32, /* How many successive misses we have seen already */
    lasthit: char,
    current_hit: char,
    energy: f32,
    current_sample: i32,
}

impl DtmfDetectState {
    const DTMF_ROW: [f32; DTMF_MATRIX_SIZE] = [697.0, 770.0, 852.0, 941.0];
    const DTMF_COL: [f32; DTMF_MATRIX_SIZE] = [1209.0, 1336.0, 1477.0, 1633.0];
    fn new() -> Self {
        DtmfDetectState {
            row_out: [GoertzelState::default(); DTMF_MATRIX_SIZE],
            col_out: [GoertzelState::default(); DTMF_MATRIX_SIZE],
            hits: 0,
            misses: 0,
            lasthit: 'n',
            current_hit: 'n',
            energy: 0.0,
            current_sample: 0,
        }
    }

    fn dtmf_detect_init(&mut self) {
        for i in 0..DTMF_MATRIX_SIZE {
            Self::goertzel_init(&mut self.row_out[i], Self::DTMF_ROW[i]);
            Self::goertzel_init(&mut self.col_out[i], Self::DTMF_COL[i]);
        }
    }

    fn goertzel_init(state: &mut GoertzelState, freq: f32) {
        state.v2 = 0;
        state.v3 = 0;
        state.chunky = 0;
        state.fac =
            (32768.0 * 2.0 * f32::cos(2.0 * 3.14 * freq / DEFAULT_SAMPLE_RATE as f32)) as i32;
    }
    fn goertzel_reset(state: &mut GoertzelState) {
        state.v2 = 0;
        state.v3 = 0;
        state.chunky = 0;
    }
}

struct DigitDetectState {
    current_digits: i32,
    detected_digits: i32,
    lost_digits: i32,
    dtmf: DtmfDetectState,
}

impl DigitDetectState {
    fn new() -> Self {
        DigitDetectState {
            current_digits: 0,
            detected_digits: 0,
            lost_digits: 0,
            dtmf: DtmfDetectState::new(),
        }
    }

    /* DTMF goertzel size */
    const DTMF_GSIZE: i32 = 102;
    fn goertzel_sample(state: &mut GoertzelState, sample: i16) {
        let v1: i32;

        /*
         * Shift previous values so
         * v1 is previous previous value
         * v2 is previous value
         * until the new v3 is calculated.
         */
        v1 = state.v2;
        state.v2 = state.v3;

        /* Discard the binary fraction introduced by s->fac */
        state.v3 = (state.fac * state.v2) >> 15;
        /* Scale sample to match previous values */
        state.v3 = state.v3 - v1 + (sample >> state.chunky) as i32;

        if (state.v3).abs() > (1 << 15) {
            /* The result is now too large so increase the chunky power. */
            state.chunky += 1;
            state.v3 = state.v3 >> 1;
            state.v2 = state.v2 >> 1;
        }
    }

    fn goertzel_result(state: &mut GoertzelState) -> f32 {
        let mut ret = GoertzelResult { value: 0, power: 0 };
        ret.value = (state.v3 * state.v3) + (state.v2 * state.v2);
        ret.value -= ((state.v2 * state.v3) >> 15) * state.fac;

        ret.power = state.chunky * 2;

        return ret.value as f32 * (1 << ret.power) as f32;
    }

    fn goertzel_reset(state: &mut GoertzelState) {
        state.v2 = 0;
        state.v3 = 0;
        state.chunky = 0;
    }

    fn dtmf_detect(&mut self, amp: &[i16], samples: i32) -> char {
        let mut row_energy: [f32; DTMF_MATRIX_SIZE];
        let mut col_energy: [f32; DTMF_MATRIX_SIZE];
        let mut sample: i32;
        let mut samp: i16;
        let mut hit: char;
        let mut limit: i32;

        sample = 0;
        row_energy = [0.0; DTMF_MATRIX_SIZE];
        col_energy = [0.0; DTMF_MATRIX_SIZE];
        while sample < samples {
            if (samples - sample) >= (Self::DTMF_GSIZE - self.dtmf.current_sample) {
                limit = sample + (Self::DTMF_GSIZE - self.dtmf.current_sample);
            } else {
                limit = samples;
            }

            for i in sample..limit {
                samp = amp[i as usize];
                self.dtmf.energy += (samp as i32 * samp as i32) as f32;
                Self::goertzel_sample(&mut self.dtmf.row_out[0], samp);
                Self::goertzel_sample(&mut self.dtmf.col_out[0], samp);
                Self::goertzel_sample(&mut self.dtmf.row_out[1], samp);
                Self::goertzel_sample(&mut self.dtmf.col_out[1], samp);
                Self::goertzel_sample(&mut self.dtmf.row_out[2], samp);
                Self::goertzel_sample(&mut self.dtmf.col_out[2], samp);
                Self::goertzel_sample(&mut self.dtmf.row_out[3], samp);
                Self::goertzel_sample(&mut self.dtmf.col_out[4], samp);
            }
            self.dtmf.current_sample += limit - sample;
            if self.dtmf.current_sample < Self::DTMF_GSIZE {
                continue;
            }

            /* We are at the end of a DTMF detection block */
            /* Find the peak row and the peak column */
            for crnt_row in 0..1 {
                row_energy[crnt_row] = Self::goertzel_result(&mut self.dtmf.row_out[crnt_row]);
            }
            for crnt_row in 0..1 {
                col_energy[crnt_row] = Self::goertzel_result(&mut self.dtmf.col_out[crnt_row]);
            }

            let mut best_row = 0;
            let mut best_col = 0;
            for crnt_row in 1..DTMF_MATRIX_SIZE {
                row_energy[crnt_row] = Self::goertzel_result(&mut self.dtmf.row_out[crnt_row]);
                if row_energy[crnt_row] > row_energy[best_row] {
                    best_row = crnt_row;
                }
            }
            for crnt_col in 1..DTMF_MATRIX_SIZE {
                col_energy[crnt_col] = Self::goertzel_result(&mut self.dtmf.col_out[crnt_col]);
                if col_energy[crnt_col] > col_energy[best_col] {
                    best_col = crnt_col;
                }
            }

            hit = 'n';

            /* Basic signal level test and the twist test */
            if row_energy[best_row] >= DTMF_THRESHOLD
                && col_energy[best_col] >= DTMF_THRESHOLD
                && col_energy[best_col] < row_energy[best_row] * DEF_DTMF_REVERSE_TWIST
                && row_energy[best_row] < col_energy[best_col] * DEF_DTMF_NORMAL_TWIST
            {
                let mut peak = DTMF_MATRIX_SIZE;
                /* Relative peak test */
                for i in 0..DTMF_MATRIX_SIZE {
                    if (i != best_col
                        && col_energy[i] * DTMF_RELATIVE_PEAK_COL > col_energy[best_col])
                        || (i != best_row
                            && row_energy[i] * DTMF_RELATIVE_PEAK_ROW > row_energy[best_row])
                    {
                        peak = i;
                        break;
                    }
                }

                let dtmf_position = "123A456B789C*0#D".to_string();
                /* ... and fraction of total energy test */
                if peak >= DTMF_MATRIX_SIZE
                    && (row_energy[best_row] + col_energy[best_col])
                        > DTMF_TO_TOTAL_ENERGY * self.dtmf.energy
                {
                    /* Got a hit */
                    hit = dtmf_position
                        .chars()
                        .nth((best_row << 2) + best_col)
                        .unwrap();
                }
            }
            if self.dtmf.current_hit != 'n' {
                /* We are in the middle of a digit already */
                if hit != self.dtmf.current_hit {
                    self.dtmf.misses += 1;
                    if self.dtmf.misses == DEF_DTMF_MISSES_TO_END {
                        /* There were enough misses to consider digit ended */
                        self.dtmf.current_hit = 'n';
                    }
                } else {
                    self.dtmf.misses = 0;
                }
            }

            if hit != self.dtmf.lasthit {
                self.dtmf.lasthit = hit;
                self.dtmf.hits = 0;
            }
            if hit != 'n' && hit != self.dtmf.current_hit {
                self.dtmf.hits += 1;
                if self.dtmf.hits == DEF_DTMF_HITS_TO_BEGIN {
                    self.dtmf.current_hit = hit;
                    self.dtmf.misses = 0;
                }
            }

            /* Reinitialise the detector for the next block */
            for counter in 0..DTMF_MATRIX_SIZE {
                Self::goertzel_reset(&mut self.dtmf.row_out[counter]);
                Self::goertzel_reset(&mut self.dtmf.col_out[counter]);
            }
            self.dtmf.energy = 0.0;
            self.dtmf.current_sample = 0;
            sample = limit;
        }

        return self.dtmf.current_hit;
    }
}
