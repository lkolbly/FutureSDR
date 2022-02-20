use std::collections::HashMap;
use std::time::Duration;

use clap::Parser;
use futuresdr::anyhow::Result;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::Apply;
use futuresdr::blocks::Filter;
use futuresdr::blocks::FirBuilder;
use futuresdr::blocks::IirBuilder;
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

const TAPS_AUDIO: [f32; 115] = [
    -7.654497174566817e-20,
    1.479578637406199e-06,
    4.49493172495973e-06,
    4.52604374626955e-06,
    -4.181240711414935e-06,
    -2.525667200343231e-05,
    -5.684918905855777e-05,
    -8.973542261930614e-05,
    -0.00010811780189899562,
    -9.362316267879954e-05,
    -3.195655503740599e-05,
    7.946503802700693e-05,
    0.00022542275706361164,
    0.0003710868133181347,
    0.00046656548147132543,
    0.000458339786669344,
    0.0003059428916626897,
    -5.847343761237861e-19,
    -0.00042386022095922935,
    -0.0008809854851106542,
    -0.0012478340055455256,
    -0.0013872653065501428,
    -0.001185544331242119,
    -0.0005930747620733689,
    0.00034237584587486377,
    0.0014618532290784584,
    0.002509935128602825,
    0.0031810008710117534,
    0.0031916134062403003,
    0.002364780919757301,
    0.0007052061186038917,
    -0.001557529636203953,
    -0.003977030619979019,
    -0.005958574743982924,
    -0.0068826551858772785,
    -0.006262275687930302,
    -0.0038994679511078294,
    3.4989379188542065e-18,
    0.004791649759202114,
    0.009461505851719412,
    0.012801970341156899,
    0.013670694826595997,
    0.011284513094513995,
    0.005484433594879865,
    -0.0030950962295271923,
    -0.01300659783027185,
    -0.022146358973898935,
    -0.028077633212196876,
    -0.028471764905923083,
    -0.02158732931313626,
    -0.006690921624430644,
    0.01567322856131063,
    0.04363339307635809,
    0.07418790453433925,
    0.10362837784672863,
    0.12811936033632507,
    0.1443301877573948,
    0.15000042329588018,
    0.1443301877573948,
    0.12811936033632507,
    0.10362837784672863,
    0.07418790453433925,
    0.04363339307635809,
    0.01567322856131063,
    -0.006690921624430644,
    -0.02158732931313626,
    -0.028471764905923083,
    -0.028077633212196876,
    -0.022146358973898935,
    -0.01300659783027185,
    -0.0030950962295271923,
    0.005484433594879865,
    0.011284513094513995,
    0.013670694826595997,
    0.012801970341156899,
    0.009461505851719412,
    0.004791649759202114,
    3.4989379188542065e-18,
    -0.0038994679511078294,
    -0.006262275687930302,
    -0.0068826551858772785,
    -0.005958574743982924,
    -0.003977030619979019,
    -0.001557529636203953,
    0.0007052061186038917,
    0.002364780919757301,
    0.0031916134062403003,
    0.0031810008710117534,
    0.002509935128602825,
    0.0014618532290784584,
    0.00034237584587486377,
    -0.0005930747620733689,
    -0.001185544331242119,
    -0.0013872653065501428,
    -0.0012478340055455256,
    -0.0008809854851106542,
    -0.00042386022095922935,
    -5.847343761237861e-19,
    0.0003059428916626897,
    0.000458339786669344,
    0.00046656548147132543,
    0.0003710868133181347,
    0.00022542275706361164,
    7.946503802700693e-05,
    -3.195655503740599e-05,
    -9.362316267879954e-05,
    -0.00010811780189899562,
    -8.973542261930614e-05,
    -5.684918905855777e-05,
    -2.525667200343231e-05,
    -4.181240711414935e-06,
    4.52604374626955e-06,
    4.49493172495973e-06,
    1.479578637406199e-06,
    -7.654497174566817e-20,
];

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Gain to apply to the soapy source, if used
    #[clap(short, long, default_value_t = 20.0)]
    gain: f64,

    /// Soapy source to use as a source
    #[clap(long)]
    soapy: Option<String>,

    /// File to use as a source
    #[clap(long)]
    filename: Option<String>,

    /// Block to probe
    #[clap(long)]
    probe: Option<String>,

    /// Volume between 0 and 1
    #[clap(long)]
    volume: Option<f32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut fg = Flowgraph::new();

    let src = if let Some(soapy) = args.soapy {
        let gain: f64 = args.gain;
        fg.add_block(SoapySource::new(105e6, 8e6, gain, soapy.to_string()))
    } else if let Some(filename) = args.filename {
        let src = fg.add_block(FileSource::<Complex<i8>>::new(filename.to_string()));

        let typecvt = fg.add_block(Apply::<Complex<i8>, Complex<f32>>::new(|i| {
            let o = Complex {
                re: i.re as f32 / 127.,
                im: i.im as f32 / 127.,
            };
            o
        }));
        fg.connect_stream(src, "out", typecvt, "in")?;
        typecvt
    } else {
        panic!("Must specify either a soapy source or a filename!");
    };

    // Center the desired band (BOB FM)
    let mut phase = Complex { re: 1.0, im: 0.0 };
    let phase_shift = Complex::from_polar(1.0, 3.14159265 * 1.5 / 4.0);
    let freqshift = fg.add_block(Apply::<Complex<f32>, Complex<f32>>::new(move |i| {
        phase = phase * phase_shift;
        phase = phase / phase.norm();
        i * phase
    }));

    // Apply some gain
    let gain = fg.add_block(Apply::<Complex<f32>, Complex<f32>>::new(|i| i * 10.0));

    // Filter out the rest
    let filter = fg.add_block(FirBuilder::new::<Complex<f32>, f32, _>(TAPS1.clone()));

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
    let filter2 = fg.add_block(FirBuilder::new::<Complex<f32>, f32, _>(TAPS2.clone()));

    // Decimate again
    let mut count = 0;
    let decimate2 = fg.add_block(Filter::<Complex<f32>, Complex<f32>>::new(move |i| {
        count += 1;
        if count == 4 {
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

    // Filter out everything >15kHz
    let audio_filter = fg.add_block(FirBuilder::new::<f32, f32, _>(TAPS_AUDIO.clone()));

    // Decimate the resulting audio
    let mut count = 0;
    let decimate_audio = fg.add_block(Filter::<f32, f32>::new(move |i| {
        count += 1;
        if count == 5 {
            count = 0;
            Some(*i)
        } else {
            None
        }
    }));

    // De-emphasis filter
    let deemphasis = fg.add_block(IirBuilder::new::<f32, f32, _>(
        vec![1.0, -0.7547],
        vec![0.12264, 0.12264],
        0.0,
    ));

    // Apply volume control
    let volume = args.volume.unwrap_or(1.0);
    let volume = fg.add_block(Apply::<f32, f32>::new(move |i| i * volume));

    // Send to audio
    let speaker = fg.add_block(AudioSink::new(40_000, 1));

    let mut blocks: HashMap<&'static str, (usize, bool)> = HashMap::new();
    blocks.insert("src", (src, true));
    blocks.insert("freqshift", (freqshift, true));
    blocks.insert("gain", (gain, true));
    blocks.insert("filter", (filter, true));
    blocks.insert("decimate", (decimate, true));
    blocks.insert("filter2", (filter2, true));
    blocks.insert("decimate2", (decimate2, true));
    blocks.insert("fm_demod", (fm_demod, false));
    blocks.insert("audio_filter", (audio_filter, false));
    blocks.insert("decimate_audio", (decimate_audio, false));
    blocks.insert("deemphasis", (deemphasis, false));
    blocks.insert("volume", (volume, false));

    let block_to_record: Option<(usize, bool)> = args.probe.map(|block_to_record| {
        *blocks
            .get(&*block_to_record)
            .expect("Didn't recognize block")
    });
    if let Some((block_to_record, record_complex)) = block_to_record {
        if record_complex {
            let typecvt = fg.add_block(Apply::<Complex<f32>, Complex<i8>>::new(|i| {
                let o = Complex {
                    re: (i.re * 127.) as i8,
                    im: (i.im * 127.) as i8,
                };
                o
            }));
            let sink = fg.add_block(FileSink::<Complex<i8>>::new("/tmp/tmp.cs8"));

            fg.connect_stream(block_to_record, "out", typecvt, "in")?;
            fg.connect_stream(typecvt, "out", sink, "in")?;
        } else {
            let typecvt = fg.add_block(Apply::<f32, Complex<f32>>::new(|sample| Complex {
                re: *sample,
                im: 0.0,
            }));
            let sink = fg.add_block(FileSink::<Complex<f32>>::new("tmp.cf32"));

            fg.connect_stream(block_to_record, "out", typecvt, "in")?;
            fg.connect_stream(typecvt, "out", sink, "in")?;
        }
    }

    fg.connect_stream(src, "out", freqshift, "in")?;
    fg.connect_stream(freqshift, "out", gain, "in")?;
    fg.connect_stream(gain, "out", filter, "in")?;
    fg.connect_stream(filter, "out", decimate, "in")?;
    fg.connect_stream(decimate, "out", filter2, "in")?;
    fg.connect_stream(filter2, "out", decimate2, "in")?;
    fg.connect_stream(decimate2, "out", fm_demod, "in")?;
    fg.connect_stream(fm_demod, "out", audio_filter, "in")?;
    fg.connect_stream(audio_filter, "out", decimate_audio, "in")?;
    fg.connect_stream(decimate_audio, "out", volume, "in")?;
    fg.connect_stream(volume, "out", deemphasis, "in")?;
    fg.connect_stream(deemphasis, "out", speaker, "in")?;

    Runtime::new().run(fg)?;

    Ok(())
}
