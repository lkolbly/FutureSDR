use futuresdr::anyhow::Result;
use futuresdr::blocks::FiniteSource;
use futuresdr::blocks::Head;
use futuresdr::blocks::VectorSink;
use futuresdr::blocks::VectorSinkBuilder;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

#[test]
fn finite_source_const_fn() -> Result<()> {
    let mut fg = Flowgraph::new();

    let src = fg.add_block(FiniteSource::new(|| Some(123u32)));
    let head = fg.add_block(Head::new(4, 10));
    let snk = fg.add_block(VectorSinkBuilder::<u32>::new().build());

    fg.connect_stream(src, "out", head, "in")?;
    fg.connect_stream(head, "out", snk, "in")?;

    fg = Runtime::new().run(fg)?;

    let snk = fg.block_async::<VectorSink<u32>>(snk).unwrap();
    let v = snk.items();

    assert_eq!(v.len(), 10);
    for i in v {
        assert_eq!(*i, 123u32);
    }

    Ok(())
}

#[test]
fn finite_source_mut_fn() -> Result<()> {
    let mut fg = Flowgraph::new();

    let mut v = vec![0, 1, 2, 3].into_iter();
    let src = fg.add_block(FiniteSource::new(move || v.next()));
    let snk = fg.add_block(VectorSinkBuilder::<u32>::new().build());

    fg.connect_stream(src, "out", snk, "in")?;

    fg = Runtime::new().run(fg)?;

    let snk = fg.block_async::<VectorSink<u32>>(snk).unwrap();
    let v = snk.items();

    assert_eq!(v.len(), 4);
    for (i, n) in v.iter().enumerate() {
        assert_eq!(i as u32, *n);
    }

    Ok(())
}
