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
use futuresdr::blocks::{FileSink, FileSource, NullSink, Source, Throttle};
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Pmt;
use futuresdr::runtime::Runtime;
use num_complex::Complex;
use rand::Rng;

fn main() -> Result<()> {
    let mut fg = Flowgraph::new();

    // Make a chirp source
    let mut freq = 0.0;
    let mut current_value = Complex::from_polar(1.0, 0.0);
    let src = fg.add_block(Source::<f32>::new(move || {
        freq += 0.00003;
        if freq > 3.14159265 {
            freq = 0.0;
        }
        current_value = current_value * Complex::from_polar(1.0, freq);
        current_value.re
    }));

    // Throttle that & send it to file
    let throttle = fg.add_block(Throttle::new(4, 100_000.0));
    let sink = fg.add_block(FileSink::new(4, "/tmp/tmp.f32"));

    fg.connect_stream(src, "out", throttle, "in")?;
    fg.connect_stream(throttle, "out", sink, "in")?;

    Runtime::new().run(fg)?;

    Ok(())
}
