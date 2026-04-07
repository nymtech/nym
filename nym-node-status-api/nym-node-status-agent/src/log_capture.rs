use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
pub(crate) struct LogCapture {
    buffer: Arc<Mutex<Vec<u8>>>,
    capturing: Arc<AtomicBool>,
}

impl LogCapture {
    pub(crate) fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn start(&self) {
        self.buffer.lock().unwrap().clear();
        self.capturing.store(true, Ordering::Release);
    }

    pub(crate) fn stop_and_drain(&self) -> String {
        self.capturing.store(false, Ordering::Release);
        let buf = std::mem::take(&mut *self.buffer.lock().unwrap());
        String::from_utf8_lossy(&buf).into_owned()
    }
}

pub(crate) struct LogCaptureWriter {
    buffer: Option<Arc<Mutex<Vec<u8>>>>,
}

impl Write for LogCaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(buffer) = &self.buffer {
            buffer.lock().unwrap().extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogCapture {
    type Writer = LogCaptureWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogCaptureWriter {
            buffer: if self.capturing.load(Ordering::Acquire) {
                Some(self.buffer.clone())
            } else {
                None
            },
        }
    }
}
