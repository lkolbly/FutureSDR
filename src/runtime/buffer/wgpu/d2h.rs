use futures::channel::mpsc::Sender;
use futures::prelude::*;
use std::any::Any;
use std::sync::{Arc, Mutex};
use wgpu::BufferView;

use crate::runtime::buffer::wgpu::OutputBufferEmpty as BufferEmpty;
use crate::runtime::buffer::wgpu::OutputBufferFull as BufferFull;
use crate::runtime::buffer::BufferBuilder;
use crate::runtime::buffer::BufferReader;
use crate::runtime::buffer::BufferReaderHost;
use crate::runtime::buffer::BufferWriter;
use crate::runtime::buffer::BufferWriterCustom;
use crate::runtime::AsyncMessage;

#[derive(Debug, PartialEq, Hash)]
pub struct D2H;

impl Eq for D2H {}

impl D2H {
    pub fn new() -> D2H {
        D2H
    }
}

impl Default for D2H {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferBuilder for D2H {
    fn build(
        &self,
        item_size: usize,
        writer_inbox: Sender<AsyncMessage>,
        writer_output_id: usize,
    ) -> BufferWriter {
        WriterD2H::new(item_size, writer_inbox, writer_output_id)
    }
}

// everything is measured in items, e.g., offsets, capacity, space available

#[derive(Debug)]
pub struct WriterD2H {
    item_size: usize,
    inbound: Arc<Mutex<Vec<BufferEmpty>>>,
    outbound: Arc<Mutex<Vec<BufferFull>>>,
    finished: bool,
    writer_inbox: Sender<AsyncMessage>,
    writer_output_id: usize,
    reader_inbox: Option<Sender<AsyncMessage>>,
    reader_input_id: Option<usize>,
}

impl WriterD2H {
    pub fn new(
        item_size: usize,
        writer_inbox: Sender<AsyncMessage>,
        writer_output_id: usize,
    ) -> BufferWriter {
        BufferWriter::Custom(Box::new(WriterD2H {
            item_size,
            outbound: Arc::new(Mutex::new(Vec::new())),
            inbound: Arc::new(Mutex::new(Vec::new())),
            finished: false,
            writer_inbox,
            writer_output_id,
            reader_inbox: None,
            reader_input_id: None,
        }))
    }

    pub fn buffers(&mut self) -> Vec<BufferEmpty> {
        let mut vec = self.inbound.lock().unwrap();
        std::mem::take(&mut vec)
    }

    pub fn submit(&mut self, buffer: BufferFull) {
        self.outbound.lock().unwrap().push(buffer);
        let _ = self
            .reader_inbox
            .as_mut()
            .unwrap()
            .try_send(AsyncMessage::Notify);
    }
}

#[async_trait]
impl BufferWriterCustom for WriterD2H {
    fn add_reader(
        &mut self,
        reader_inbox: Sender<AsyncMessage>,
        reader_input_id: usize,
    ) -> BufferReader {
        debug_assert!(self.reader_inbox.is_none());
        debug_assert!(self.reader_input_id.is_none());

        self.reader_inbox = Some(reader_inbox);
        self.reader_input_id = Some(reader_input_id);

        BufferReader::Host(Box::new(ReaderD2H {
            buffer: None,
            outbound: self.inbound.clone(),
            inbound: self.outbound.clone(),
            item_size: self.item_size,
            writer_inbox: self.writer_inbox.clone(),
            writer_output_id: self.writer_output_id,
            finished: false,
        }))
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    async fn notify_finished(&mut self) {
        if self.finished {
            return;
        }

        self.reader_inbox
            .as_mut()
            .unwrap()
            .send(AsyncMessage::StreamInputDone {
                input_id: self.reader_input_id.unwrap(),
            })
            .await
            .unwrap();
    }

    fn finish(&mut self) {
        self.finished = true;
    }

    fn finished(&self) -> bool {
        self.finished
    }
}

unsafe impl Send for WriterD2H {}

#[derive(Debug)]
pub struct ReaderD2H {
    buffer: Option<CurrentBuffer>,
    inbound: Arc<Mutex<Vec<BufferFull>>>,
    outbound: Arc<Mutex<Vec<BufferEmpty>>>,
    item_size: usize,
    writer_inbox: Sender<AsyncMessage>,
    writer_output_id: usize,
    finished: bool,
}

#[derive(Debug)]
struct CurrentBuffer {
    buffer: *mut BufferFull,
    offset: usize,
    slice: BufferView<'static>,
}

#[async_trait]
impl BufferReaderHost for ReaderD2H {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn bytes(&mut self) -> (*const u8, usize) {
        debug!("D2H reader bytes");
        if self.buffer.is_none() {
            if let Some(b) = self.inbound.lock().unwrap().pop() {
                debug!("set gpuBuffer full from inbound");
                let buffer = Box::leak(Box::new(b));
                let t = buffer as *mut BufferFull;
                let slice = buffer
                    .buffer
                    .slice(0..buffer.used_bytes as u64)
                    .get_mapped_range();
                self.buffer = Some(CurrentBuffer {
                    buffer: t,
                    offset: 0,
                    slice,
                });
            } else {
                debug!("set wrong pointer");
                return (std::ptr::null::<u8>(), 0);
            }
        }

        unsafe {
            let buffer = self.buffer.as_ref().unwrap();
            let capacity = buffer.slice.len() / self.item_size;
            let ptr = buffer.slice.as_ptr();

            (
                ptr.add(buffer.offset * self.item_size),
                (capacity - buffer.offset) * self.item_size,
            )
        }
    }

    fn consume(&mut self, amount: usize) {
        let buffer = self.buffer.as_mut().unwrap();
        let capacity = buffer.slice.len() / self.item_size;
        log::info!(
            "Consume -- capacity: {}, offset: {}",
            capacity,
            buffer.offset
        );
        debug_assert!(amount + buffer.offset <= capacity);
        debug_assert!(amount != 0);

        buffer.offset += amount;
        if buffer.offset == capacity {
            let c = unsafe { Box::from_raw(self.buffer.take().unwrap().buffer) };
            let buffer = c.buffer;
            buffer.unmap();
            self.outbound.lock().unwrap().push(BufferEmpty { buffer });

            if let Some(b) = self.inbound.lock().unwrap().pop() {
                let buffer = Box::leak(Box::new(b));
                let t = buffer as *mut BufferFull;
                let slice = buffer
                    .buffer
                    .slice(0..buffer.used_bytes as u64)
                    .get_mapped_range();
                self.buffer = Some(CurrentBuffer {
                    buffer: t,
                    offset: 0,
                    slice,
                });
            }

            let _ = self.writer_inbox.try_send(AsyncMessage::Notify);
        }
    }

    async fn notify_finished(&mut self) {
        debug!("D2H Reader finish");
        if self.finished {
            return;
        }

        self.writer_inbox
            .send(AsyncMessage::StreamOutputDone {
                output_id: self.writer_output_id,
            })
            .await
            .unwrap();
    }

    fn finish(&mut self) {
        self.finished = true;
    }

    fn finished(&self) -> bool {
        self.finished
    }
}

unsafe impl Send for ReaderD2H {}
