use std::collections::HashMap;
use std::time::Duration;

use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::Apply;
use futuresdr::blocks::ComplexFir;
use futuresdr::blocks::Filter;
use futuresdr::blocks::Fir;
use futuresdr::blocks::MessageSink;
use futuresdr::blocks::MessageSource;
use futuresdr::blocks::SoapySource;
use futuresdr::blocks::{FileSink, FileSource, Source};
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Pmt;
use futuresdr::runtime::Runtime;
use num_complex::Complex;
use rand::Rng;

const TAPS1: [f32; 59] = [
    -3.200768627064545e-19,
    2.5695329810174536e-05,
    0.0001090015867311698,
    0.00026138383446949726,
    0.0004972841870069384,
    0.0008340340557757516,
    0.0012914444095230506,
    0.0018910706457681734,
    0.00265517309506422,
    0.003605418278370962,
    0.0047613881109414835,
    0.00613898268428003,
    0.007748815659053333,
    0.009594708572398905,
    0.011672390798343847,
    0.013968505240990886,
    0.016460006299700354,
    0.019114016903342565,
    0.021888186575993954,
    0.024731564041054283,
    0.027585967536853464,
    0.03038780570507866,
    0.03307027355907554,
    0.035565823484471785,
    0.03780879209592808,
    0.039738051371860414,
    0.04129954769989685,
    0.04244859569921351,
    0.04315180484381208,
    0.04338853539038002,
    0.04315180484381208,
    0.04244859569921351,
    0.04129954769989685,
    0.039738051371860414,
    0.03780879209592808,
    0.035565823484471785,
    0.03307027355907554,
    0.03038780570507866,
    0.027585967536853464,
    0.024731564041054283,
    0.021888186575993954,
    0.019114016903342565,
    0.016460006299700354,
    0.013968505240990886,
    0.011672390798343847,
    0.009594708572398905,
    0.007748815659053333,
    0.00613898268428003,
    0.0047613881109414835,
    0.003605418278370962,
    0.00265517309506422,
    0.0018910706457681734,
    0.0012914444095230506,
    0.0008340340557757516,
    0.0004972841870069384,
    0.00026138383446949726,
    0.0001090015867311698,
    2.5695329810174536e-05,
    -3.200768627064545e-19,
];

const TAPS2: [f32; 115] = [
    -5.649291030560293e-20,
    3.8688173114554383e-07,
    -1.9643045920455557e-06,
    -1.1285085688331897e-05,
    -2.6516914275716006e-05,
    -3.8878668784018145e-05,
    -3.4186621832351765e-05,
    9.890212985055958e-20,
    6.501732862114954e-05,
    0.00014411811389112027,
    0.00020266454121158376,
    0.00019813546082059886,
    9.851071917128737e-05,
    -9.703215842145873e-05,
    -0.00034434191162846484,
    -0.0005564892657111382,
    -0.0006265569208376126,
    -0.00046773899714766076,
    -5.8621691048703836e-05,
    0.0005245954440789613,
    0.0011070860603557542,
    0.0014557397562067335,
    0.0013563719852489715,
    0.0007064968570145578,
    -0.00041009580695738136,
    -0.0017024595239806264,
    -0.0027283928196230665,
    -0.0030252305964493706,
    -0.002283211648270276,
    -0.0005042278650636374,
    0.0019193604250874396,
    0.004255528260303984,
    0.005624220034189772,
    0.005289755392029652,
    0.002966941551752128,
    -0.000970128254424872,
    -0.0054748895895182295,
    -0.009057301536556777,
    -0.01022265222756236,
    -0.00800560811432811,
    -0.0024286869633116386,
    0.0052913613160398484,
    0.012910523061198054,
    0.017712503998204742,
    0.01733748936502409,
    0.010660030139764758,
    -0.001560644183424048,
    -0.016503176795836204,
    -0.02970984828805608,
    -0.03607506391697371,
    -0.0311781764012777,
    -0.012613122063709402,
    0.01906798136899112,
    0.06010287637222585,
    0.10409000479418232,
    0.1432884102295912,
    0.17034257568219163,
    0.17999568799232507,
    0.17034257568219163,
    0.1432884102295912,
    0.10409000479418232,
    0.06010287637222585,
    0.01906798136899112,
    -0.012613122063709402,
    -0.0311781764012777,
    -0.03607506391697371,
    -0.02970984828805608,
    -0.016503176795836204,
    -0.001560644183424048,
    0.010660030139764758,
    0.01733748936502409,
    0.017712503998204742,
    0.012910523061198054,
    0.0052913613160398484,
    -0.0024286869633116386,
    -0.00800560811432811,
    -0.01022265222756236,
    -0.009057301536556777,
    -0.0054748895895182295,
    -0.000970128254424872,
    0.002966941551752128,
    0.005289755392029652,
    0.005624220034189772,
    0.004255528260303984,
    0.0019193604250874396,
    -0.0005042278650636374,
    -0.002283211648270276,
    -0.0030252305964493706,
    -0.0027283928196230665,
    -0.0017024595239806264,
    -0.00041009580695738136,
    0.0007064968570145578,
    0.0013563719852489715,
    0.0014557397562067335,
    0.0011070860603557542,
    0.0005245954440789613,
    -5.8621691048703836e-05,
    -0.00046773899714766076,
    -0.0006265569208376126,
    -0.0005564892657111382,
    -0.00034434191162846484,
    -9.703215842145873e-05,
    9.851071917128737e-05,
    0.00019813546082059886,
    0.00020266454121158376,
    0.00014411811389112027,
    6.501732862114954e-05,
    9.890212985055958e-20,
    -3.4186621832351765e-05,
    -3.8878668784018145e-05,
    -2.6516914275716006e-05,
    -1.1285085688331897e-05,
    -1.9643045920455557e-06,
    3.8688173114554383e-07,
    -5.649291030560293e-20,
];

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    let src = fg.add_block(Source::<Complex<i8>>::new(|| {
        let mut rng = rand::thread_rng();
        Complex {
            re: rng.gen(),
            im: rng.gen(),
        }
    }));
    //let src = fg.add_block(FileSource::new(2, "fm.cs8".to_string()));
    /*let src = fg.add_block(SoapySource::new(
        100e6,
        8e6,
        20.0,
        "driver=hackrf".to_string(),
    ));*/

    let typecvt0 = fg.add_block(Apply::<Complex<i8>, Complex<f32>>::new(|i| {
        let o = Complex {
            re: i.re as f32 / 127.,
            im: i.im as f32 / 127.,
        };
        o
    }));

    // Center the desired band (BOB FM)
    let mut phase = Complex { re: 1.0, im: 0.0 };
    let phase_shift = Complex::from_polar(1.0, 3.14159265 * -3.5 / 4.0);
    let freqshift = fg.add_block(Apply::<Complex<f32>, Complex<f32>>::new(move |i| {
        phase = phase * phase_shift;
        i * phase
    }));

    // Apply some gain
    let gain = fg.add_block(Apply::<Complex<f32>, Complex<f32>>::new(|i| i * 10.0));

    // Filter out the rest
    let filter = fg.add_block(ComplexFir::new(&TAPS1, 1));

    // Decimate
    let mut count = 0;
    let decimate = fg.add_block(Filter::<Complex<f32>, Complex<f32>>::new(move |i| {
        count += 1;
        if count == 10 {
            count = 0;
            Some(*i)
        } else {
            None
        }
    }));

    // Filter again w/ a steeper filter
    let filter2 = fg.add_block(ComplexFir::new(&TAPS2, 1));

    // Decimate again
    let mut count = 0;
    let decimate2 = fg.add_block(Filter::<Complex<f32>, Complex<f32>>::new(move |i| {
        count += 1;
        if count == 20 {
            count = 0;
            Some(*i)
        } else {
            None
        }
    }));

    // FM demod
    let mut previous = Complex::from_polar(1.0, 0.0);
    let fm_demod = fg.add_block(Apply::<Complex<f32>, f32>::new(move |i| {
        let diff = i * previous.conj();
        let (_, phase) = diff.to_polar();
        previous = *i;
        phase
    }));

    // Send to audio
    let speaker = fg.add_block(AudioSink::new(40_000, 1));

    // Capture the results in a file
    let typecvt = fg.add_block(Apply::<Complex<f32>, Complex<i8>>::new(|i| {
        let o = Complex {
            re: (i.re * 127.) as i8,
            im: (i.im * 127.) as i8,
        };
        o
    }));
    let sink = fg.add_block(FileSink::new(2, "/tmp/tmp.cs8"));
    /*let typecvt = fg.add_block(Apply::<f32, i8>::new(|i| (i * 127.) as i8));
    let sink = fg.add_block(FileSink::new(1, "tmp.s8"));*/

    let mut blocks: HashMap<&'static str, usize> = HashMap::new();
    blocks.insert("src", src);
    blocks.insert("typecvt0", typecvt0);
    blocks.insert("freqshift", freqshift);
    blocks.insert("gain", gain);
    blocks.insert("filter", filter);
    blocks.insert("decimate", decimate);
    blocks.insert("filter2", filter2);
    blocks.insert("decimate2", decimate2);

    fg.connect_stream(src, "out", typecvt0, "in")?;
    fg.connect_stream(typecvt0, "out", freqshift, "in")?;
    fg.connect_stream(freqshift, "out", gain, "in")?;
    fg.connect_stream(gain, "out", filter, "in")?;
    fg.connect_stream(filter, "out", decimate, "in")?;
    fg.connect_stream(decimate, "out", filter2, "in")?;
    fg.connect_stream(filter2, "out", decimate2, "in")?;
    fg.connect_stream(decimate2, "out", fm_demod, "in")?;
    fg.connect_stream(fm_demod, "out", speaker, "in")?;

    let block_to_record: String = std::env::args()
        .nth(1)
        .unwrap_or("typecvt0".to_string())
        .to_owned();
    fg.connect_stream(
        *blocks.get(&*block_to_record).unwrap(),
        "out",
        typecvt,
        "in",
    )?;
    fg.connect_stream(typecvt, "out", sink, "in")?;

    Runtime::new().run(fg)?;

    Ok(())
}
