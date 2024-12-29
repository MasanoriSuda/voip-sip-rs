

#define MAX_DTMF_DIGITS 128

#define DTMF_MATRIX_SIZE 4

/* Basic DTMF (AT&T) specs:
 *
 * Minimum tone on = 40ms
 * Minimum tone off = 50ms
 * Maximum digit rate = 10 per second
 * Normal twist <= 8dB accepted
 * Reverse twist <= 4dB accepted
 * S/N >= 15dB will detect OK
 * Attenuation <= 26dB will detect OK
 * Frequency tolerance +- 1.5% will detect, +-3.5% will reject
 */

#define DTMF_THRESHOLD 8.0e7

#define DEF_DTMF_NORMAL_TWIST 6.31  /* 8.0dB */
#define DEF_DTMF_REVERSE_TWIST 2.51 /* 4.01dB */

#define DTMF_RELATIVE_PEAK_ROW 6.3 /* 8dB */
#define DTMF_RELATIVE_PEAK_COL 6.3 /* 8dB */
#define DTMF_TO_TOTAL_ENERGY 42.0

#define DEFAULT_SAMPLE_RATE 8000

/* DTMF goertzel size */
#define DTMF_GSIZE 102

/* How many successive hits needed to consider begin of a digit
 * IE. Override with dtmf_hits_to_begin=4 in dsp.conf
 */
#define DEF_DTMF_HITS_TO_BEGIN 2

/* How many successive misses needed to consider end of a digit
 * IE. Override with dtmf_misses_to_end=4 in dsp.conf
 */
#define DEF_DTMF_MISSES_TO_END 3

typedef struct
{
    /*! The previous previous sample calculation (No binary point just plain int) */
    int v2;
    /*! The previous sample calculation (No binary point just plain int) */
    int v3;
    /*! v2 and v3 power of two exponent to keep value in int range */
    int chunky;
    /*! 15 bit fixed point goertzel coefficient = 2 * cos(2 * pi * freq / sample_rate) */
    int fac;
} goertzel_state_t;

typedef struct
{
    int value;
    int power;
} goertzel_result_t;

typedef struct
{
    goertzel_state_t row_out[DTMF_MATRIX_SIZE];
    goertzel_state_t col_out[DTMF_MATRIX_SIZE];
    int hits;   /* How many successive hits we have seen already */
    int misses; /* How many successive misses we have seen already */
    int lasthit;
    int current_hit;
    float energy;
    int current_sample;
    int mute_samples;
} dtmf_detect_state_t;

typedef struct
{
    char digits[MAX_DTMF_DIGITS + 1];
    int digitlen[MAX_DTMF_DIGITS + 1];
    int current_digits;
    int detected_digits;
    int lost_digits;
    dtmf_detect_state_t dtmf;
} digit_detect_state_t;

static const float dtmf_row[] = {
    697.0, 770.0, 852.0, 941.0};
static const float dtmf_col[] = {
    1209.0, 1336.0, 1477.0, 1633.0};
static const char dtmf_positions[] = "123A"
                                     "456B"
                                     "789C"
                                     "*0#D";
static float dtmf_normal_twist;        /* AT&T = 8dB */
static float dtmf_reverse_twist;       /* AT&T = 4dB */
static float relax_dtmf_normal_twist;  /* AT&T = 8dB */
static float relax_dtmf_reverse_twist; /* AT&T = 6dB */
static int dtmf_hits_to_begin;         /* How many successive hits needed to consider begin of a digit */
static int dtmf_misses_to_end;         /* How many successive misses needed to consider end of a digit */

static inline void goertzel_sample(goertzel_state_t *s, short sample)
{
    int v1;

    /*
     * Shift previous values so
     * v1 is previous previous value
     * v2 is previous value
     * until the new v3 is calculated.
     */
    v1 = s->v2;
    s->v2 = s->v3;

    /* Discard the binary fraction introduced by s->fac */
    s->v3 = (s->fac * s->v2) >> 15;
    /* Scale sample to match previous values */
    s->v3 = s->v3 - v1 + (sample >> s->chunky);

    if (abs(s->v3) > (1 << 15))
    {
        /* The result is now too large so increase the chunky power. */
        s->chunky++;
        s->v3 = s->v3 >> 1;
        s->v2 = s->v2 >> 1;
    }
}

static inline float goertzel_result(goertzel_state_t *s)
{
    goertzel_result_t r;

    r.value = (s->v3 * s->v3) + (s->v2 * s->v2);
    r.value -= ((s->v2 * s->v3) >> 15) * s->fac;
    /*
     * We have to double the exponent because we multiplied the
     * previous sample calculation values together.
     */
    r.power = s->chunky * 2;
    return (float)r.value * (float)(1 << r.power);
}

static inline void goertzel_init(goertzel_state_t *s, float freq, unsigned int sample_rate)
{
    s->v2 = s->v3 = s->chunky = 0;
    s->fac = (int)(32768.0 * 2.0 * cos(2.0 * 3.14 * freq / sample_rate));
}

static inline void goertzel_reset(goertzel_state_t *s)
{
    s->v2 = s->v3 = s->chunky = 0;
}

static void ast_dtmf_detect_init(dtmf_detect_state_t *s, unsigned int sample_rate)
{
    int i;

    for (i = 0; i < DTMF_MATRIX_SIZE; i++)
    {
        goertzel_init(&s->row_out[i], dtmf_row[i], sample_rate);
        goertzel_init(&s->col_out[i], dtmf_col[i], sample_rate);
    }
    s->lasthit = 0;
    s->current_hit = 0;
    s->energy = 0.0;
    s->current_sample = 0;
    s->hits = 0;
    s->misses = 0;
}

static int dtmf_detect(digit_detect_state_t *s, short amp[], int samples, int squelch, int relax)
{
    float row_energy[DTMF_MATRIX_SIZE];
    float col_energy[DTMF_MATRIX_SIZE];
    int i;
    int j;
    int sample;
    short samp;
    int best_row;
    int best_col;
    int hit;
    int limit;

    hit = 0;
    for (sample = 0; sample < samples; sample = limit)
    {
        /* DTMF_GSIZE is optimised to meet the DTMF specs. */
        if ((samples - sample) >= (DTMF_GSIZE - s->dtmf.current_sample))
        {
            limit = sample + (DTMF_GSIZE - s->dtmf.current_sample);
        }
        else
        {
            limit = samples;
        }
        /* The following unrolled loop takes only 35% (rough estimate) of the
           time of a rolled loop on the machine on which it was developed */
        for (j = sample; j < limit; j++)
        {
            samp = amp[j];
            s->dtmf.energy += (int)samp * (int)samp;
            /* With GCC 2.95, the following unrolled code seems to take about 35%
               (rough estimate) as long as a neat little 0-3 loop */
            goertzel_sample(s->dtmf.row_out, samp);
            goertzel_sample(s->dtmf.col_out, samp);
            goertzel_sample(s->dtmf.row_out + 1, samp);
            goertzel_sample(s->dtmf.col_out + 1, samp);
            goertzel_sample(s->dtmf.row_out + 2, samp);
            goertzel_sample(s->dtmf.col_out + 2, samp);
            goertzel_sample(s->dtmf.row_out + 3, samp);
            goertzel_sample(s->dtmf.col_out + 3, samp);
            /* go up to DTMF_MATRIX_SIZE - 1 */
        }
        s->dtmf.current_sample += (limit - sample);
        if (s->dtmf.current_sample < DTMF_GSIZE)
        {
            continue;
        }
        /* We are at the end of a DTMF detection block */
        /* Find the peak row and the peak column */
        row_energy[0] = goertzel_result(&s->dtmf.row_out[0]);
        col_energy[0] = goertzel_result(&s->dtmf.col_out[0]);

        for (best_row = best_col = 0, i = 1; i < DTMF_MATRIX_SIZE; i++)
        {
            row_energy[i] = goertzel_result(&s->dtmf.row_out[i]);
            if (row_energy[i] > row_energy[best_row])
            {
                best_row = i;
            }
            col_energy[i] = goertzel_result(&s->dtmf.col_out[i]);
            if (col_energy[i] > col_energy[best_col])
            {
                best_col = i;
            }
        }

        hit = 0;
        /* Basic signal level test and the twist test */
        if (row_energy[best_row] >= DTMF_THRESHOLD &&
            col_energy[best_col] >= DTMF_THRESHOLD &&
            col_energy[best_col] < row_energy[best_row] * (relax ? relax_dtmf_reverse_twist : dtmf_reverse_twist) &&
            row_energy[best_row] < col_energy[best_col] * (relax ? relax_dtmf_normal_twist : dtmf_normal_twist))
        {
            /* Relative peak test */
            for (i = 0; i < DTMF_MATRIX_SIZE; i++)
            {
                if ((i != best_col &&
                     col_energy[i] * DTMF_RELATIVE_PEAK_COL > col_energy[best_col]) ||
                    (i != best_row && row_energy[i] * DTMF_RELATIVE_PEAK_ROW > row_energy[best_row]))
                {
                    break;
                }
            }
            /* ... and fraction of total energy test */
            if (i >= DTMF_MATRIX_SIZE &&
                (row_energy[best_row] + col_energy[best_col]) > DTMF_TO_TOTAL_ENERGY * s->dtmf.energy)
            {
                /* Got a hit */
                hit = dtmf_positions[(best_row << 2) + best_col];
            }
        }

        /*
         * Adapted from ETSI ES 201 235-3 V1.3.1 (2006-03)
         * (40ms reference is tunable with hits_to_begin and misses_to_end)
         * each hit/miss is 12.75ms with DTMF_GSIZE at 102
         *
         * Character recognition: When not DRC *(1) and then
         *      Shall exist VSC > 40 ms (hits_to_begin)
         *      May exist 20 ms <= VSC <= 40 ms
         *      Shall not exist VSC < 20 ms
         *
         * Character recognition: When DRC and then
         *      Shall cease Not VSC > 40 ms (misses_to_end)
         *      May cease 20 ms >= Not VSC >= 40 ms
         *      Shall not cease Not VSC < 20 ms
         *
         * *(1) or optionally a different digit recognition condition
         *
         * Legend: VSC The continuous existence of a valid signal condition.
         *      Not VSC The continuous non-existence of valid signal condition.
         *      DRC The existence of digit recognition condition.
         *      Not DRC The non-existence of digit recognition condition.
         */

        /*
         * Example: hits_to_begin=2 misses_to_end=3
         * -------A last_hit=A hits=0&1
         * ------AA hits=2 current_hit=A misses=0       BEGIN A
         * -----AA- misses=1 last_hit=' ' hits=0
         * ----AA-- misses=2
         * ---AA--- misses=3 current_hit=' '            END A
         * --AA---B last_hit=B hits=0&1
         * -AA---BC last_hit=C hits=0&1
         * AA---BCC hits=2 current_hit=C misses=0       BEGIN C
         * A---BCC- misses=1 last_hit=' ' hits=0
         * ---BCC-C misses=0 last_hit=C hits=0&1
         * --BCC-CC misses=0
         *
         * Example: hits_to_begin=3 misses_to_end=2
         * -------A last_hit=A hits=0&1
         * ------AA hits=2
         * -----AAA hits=3 current_hit=A misses=0       BEGIN A
         * ----AAAB misses=1 last_hit=B hits=0&1
         * ---AAABB misses=2 current_hit=' ' hits=2     END A
         * --AAABBB hits=3 current_hit=B misses=0       BEGIN B
         * -AAABBBB misses=0
         *
         * Example: hits_to_begin=2 misses_to_end=2
         * -------A last_hit=A hits=0&1
         * ------AA hits=2 current_hit=A misses=0       BEGIN A
         * -----AAB misses=1 hits=0&1
         * ----AABB misses=2 current_hit=' ' hits=2 current_hit=B misses=0 BEGIN B
         * ---AABBB misses=0
         */

        if (s->dtmf.current_hit)
        {
            /* We are in the middle of a digit already */
            if (hit != s->dtmf.current_hit)
            {
                s->dtmf.misses++;
                if (s->dtmf.misses == dtmf_misses_to_end)
                {
                    /* There were enough misses to consider digit ended */
                    s->dtmf.current_hit = 0;
                }
            }
            else
            {
                s->dtmf.misses = 0;
                /* Current hit was same as last, so increment digit duration (of last digit) */
                s->digitlen[s->current_digits - 1] += DTMF_GSIZE;
            }
        }

        /* Look for a start of a new digit no matter if we are already in the middle of some
           digit or not. This is because hits_to_begin may be smaller than misses_to_end
           and we may find begin of new digit before we consider last one ended. */

        if (hit != s->dtmf.lasthit)
        {
            s->dtmf.lasthit = hit;
            s->dtmf.hits = 0;
        }
        if (hit && hit != s->dtmf.current_hit)
        {
            s->dtmf.hits++;
            if (s->dtmf.hits == dtmf_hits_to_begin)
            {
                store_digit(s, hit);
                s->digitlen[s->current_digits - 1] = dtmf_hits_to_begin * DTMF_GSIZE;
                s->dtmf.current_hit = hit;
                s->dtmf.misses = 0;
            }
        }

        /* Reinitialise the detector for the next block */
        for (i = 0; i < DTMF_MATRIX_SIZE; i++)
        {
            goertzel_reset(&s->dtmf.row_out[i]);
            goertzel_reset(&s->dtmf.col_out[i]);
        }
        s->dtmf.energy = 0.0;
        s->dtmf.current_sample = 0;
    }

    return (s->dtmf.current_hit); /* return the debounced hit */
}
