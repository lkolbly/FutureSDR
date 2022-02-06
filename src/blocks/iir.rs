use std::mem;

use crate::anyhow::Result;
use crate::runtime::Block;
use crate::runtime::BlockMeta;
use crate::runtime::BlockMetaBuilder;
use crate::runtime::MessageIo;
use crate::runtime::MessageIoBuilder;
use crate::runtime::StreamIo;
use crate::runtime::StreamIoBuilder;
use crate::runtime::SyncKernel;
use crate::runtime::WorkIo;
use num_complex::Complex;

pub trait IirKernel<SampleType> {
    /// Returns (samples consumed, samples produced)
    fn work(&mut self, input: &[SampleType], output: &mut [SampleType]) -> (usize, usize);
}

pub trait TapsAccessor {
    type TapType;

    fn num_taps(&self) -> usize;
    fn get(&self, index: usize) -> Self::TapType;

    // TODO: This is just because I was lazy during implementation
    fn len(&self) -> usize {
        self.num_taps()
    }
}

impl<const N: usize> TapsAccessor for [f32; N] {
    type TapType = f32;

    fn num_taps(&self) -> usize {
        N
    }

    fn get(&self, index: usize) -> f32 {
        // TODO: This unsafe should make the function unsafe
        unsafe { *self.get_unchecked(index) }
    }
}

impl TapsAccessor for Vec<f32> {
    type TapType = f32;

    fn num_taps(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> f32 {
        // TODO: This unsafe should make the function unsafe
        unsafe { *self.get_unchecked(index) }
    }
}

pub struct IirKernelCore<SampleType, TapsType: TapsAccessor> {
    a_taps: TapsType,
    b_taps: TapsType,
    memory: Vec<SampleType>,
    _sampletype: std::marker::PhantomData<SampleType>,
}

impl<TapsType: TapsAccessor<TapType = f32>> IirKernel<f32> for IirKernelCore<f32, TapsType> {
    fn work(&mut self, i: &[f32], o: &mut [f32]) -> (usize, usize) {
        if i.len() == 0 {
            return (0, 0);
        }
        if self.memory.len() != self.a_taps.num_taps() {
            self.memory.push(0.0); // Make it longer & shift old values into it
            for idx in 1..self.memory.len() {
                self.memory[idx] = self.memory[idx - 1];
            }
            if self.memory.len() > 0 {
                self.memory[0] = i[self.memory.len() - 1];
            }
            return (0, 0);
        }

        assert_eq!(self.a_taps.num_taps(), self.memory.len());
        assert!(self.b_taps.len() > 0);

        let mut n_consumed = 0;
        let mut n_produced = 0;
        //println!("{} + {} - 1 <? {}", n_consumed, self.b_taps.len(), i.len());
        while n_consumed + self.b_taps.len() - 1 < i.len() && n_produced < o.len() {
            //let i = i[n_consumed];
            //println!("Computing...");
            let o: &mut f32 = &mut o[n_produced];

            *o = 0.0;

            // Calculate the intermediate value
            /*if self.b_taps.num_taps() > 0 {
                // Safety: We just checked that b_taps has at least one tap
                *o += unsafe { self.b_taps.get(0) } * i[n_consumed + self.b_taps.len() - 1];
            }*/
            for b_tap in 0..self.b_taps.num_taps() {
                // Safety: We're iterating only up to the # of taps in B
                *o += unsafe { self.b_taps.get(b_tap) }
                    * i[n_consumed + self.b_taps.len() - b_tap - 1];
            }

            // Apply the feedback a taps
            let intermediate_value: f32 = *o;
            for a_tap in 0..self.a_taps.num_taps() {
                // Safety: The iterand is limited to a_taps' length
                *o += unsafe { self.a_taps.get(a_tap) } * self.memory[a_tap];
            }

            // Update the memory
            for idx in 1..self.memory.len() {
                self.memory[idx] = self.memory[idx - 1];
            }
            if self.memory.len() > 0 {
                self.memory[0] = *o;
            }

            n_produced += 1;
            n_consumed += 1;
        }

        //let n = std::cmp::min((i.len() + 1).saturating_sub(self.taps.num_taps()), o.len());

        // Compute the intermediate value after the b taps are applies
        /*unsafe {
            for k in 0..n {
                let mut sum = 0.0;
                for t in 0..self.taps.num_taps() {
                    sum += i.get_unchecked(k + t) * self.taps.get(t);
                }
                *o.get_unchecked_mut(k) = sum;
            }
        }*/

        //(n, n)
        (n_consumed, n_produced)
    }
}

//pub trait HasFirImpl: Copy + Send + 'static {}
//impl HasFirImpl for f32 {}

pub struct Iir<SampleType, TapType, Core>
where
    SampleType: 'static + Send,
    TapType: 'static,
    Core: 'static + IirKernel<SampleType>,
{
    core: Core,
    _sampletype: std::marker::PhantomData<SampleType>,
    _taptype: std::marker::PhantomData<TapType>,
    //taps: [A; N],
}

// TODO: Determine exactly when Iir is Send
unsafe impl<SampleType, TapType, Core> Send for Iir<SampleType, TapType, Core>
where
    SampleType: 'static + Send,
    TapType: 'static,
    Core: 'static + IirKernel<SampleType>,
{
}

impl<SampleType, TapType, Core> Iir<SampleType, TapType, Core>
where
    SampleType: 'static + Send,
    TapType: 'static,
    Core: 'static + IirKernel<SampleType>,
{
    pub fn new(core: Core) -> Block {
        Block::new_sync(
            BlockMetaBuilder::new("Fir").build(),
            StreamIoBuilder::new()
                .add_input("in", mem::size_of::<SampleType>())
                .add_output("out", mem::size_of::<SampleType>())
                .build(),
            MessageIoBuilder::<Iir<SampleType, TapType, Core>>::new().build(),
            Iir {
                core: core,
                _sampletype: std::marker::PhantomData,
                _taptype: std::marker::PhantomData,
            },
        )
    }
}

#[async_trait]
impl<SampleType, TapType, Core> SyncKernel for Iir<SampleType, TapType, Core>
where
    SampleType: 'static + Send,
    TapType: 'static,
    Core: 'static + IirKernel<SampleType>,
{
    fn work(
        &mut self,
        io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<()> {
        let i = sio.input(0).slice::<SampleType>();
        let o = sio.output(0).slice::<SampleType>();

        let (consumed, produced) = self.core.work(i, o);

        sio.input(0).consume(consumed);
        sio.output(0).produce(produced);

        if consumed == 0 && sio.input(0).finished() {
            io.finished = true;
        }

        Ok(())
    }
}

pub struct IirBuilder {
    //
}

impl IirBuilder {
    // TODO: Having to pass zero as an initial sample type is weird
    pub fn new<SampleType, TapType, Taps>(a_taps: Taps, b_taps: Taps, zero: SampleType) -> Block
    where
        SampleType: 'static + Send + Clone,
        TapType: 'static,
        Taps: 'static + TapsAccessor,
        IirKernelCore<SampleType, Taps>: IirKernel<SampleType>,
    {
        let mem_length = std::cmp::min(a_taps.len(), b_taps.len());
        Iir::<SampleType, TapType, IirKernelCore<SampleType, Taps>>::new(IirKernelCore {
            a_taps,
            b_taps,
            memory: vec![],
            _sampletype: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Feeder {
        filter: IirKernelCore<f32, Vec<f32>>,
        input: Vec<f32>,
    }

    impl Feeder {
        fn feed(&mut self, input: f32) -> Option<f32> {
            self.input.push(input);

            let mut out = [0.0];
            println!("Feeding with {:?}...", self.input);
            let (n_consumed, n_produced) = self.filter.work(&self.input[..], &mut out);
            assert_eq!(n_consumed, n_produced); // If we consume samples, we produce samples
            if n_consumed > 0 {
                self.input.drain(0..n_consumed);
            }
            if n_produced > 0 {
                Some(out[0])
            } else {
                None
            }
        }
    }

    fn make_filter(a_taps: Vec<f32>, b_taps: Vec<f32>) -> Feeder {
        let memlen = a_taps.len();
        Feeder {
            filter: IirKernelCore {
                a_taps,
                b_taps,
                memory: vec![],
                _sampletype: std::marker::PhantomData,
            },
            input: vec![],
        }
    }

    #[test]
    fn test_iir_b_taps_algorithm() {
        let mut iir = make_filter(vec![], vec![1.0, 2.0, 3.0]);

        assert_eq!(iir.feed(10.0), None);
        assert_eq!(iir.feed(20.0), None);
        assert_eq!(iir.feed(30.0), Some(30.0 + 40.0 + 30.0));
        assert_eq!(iir.feed(40.0), Some(40.0 + 60.0 + 60.0));
    }

    #[test]
    fn test_iir_single_a_tap_algorithm() {
        let mut iir = make_filter(vec![0.5], vec![1.0]);

        assert_eq!(iir.feed(10.0), None);
        assert_eq!(iir.feed(10.0), Some(15.0));
        assert_eq!(iir.feed(10.0), Some(17.5));
        assert_eq!(iir.feed(10.0), Some(18.75));
    }
}
