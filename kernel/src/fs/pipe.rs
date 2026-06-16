//! Implement INode for Pipe

use crate::sync::{Event, EventBus, SpinNoIrqLock as Mutex};
use alloc::boxed::Box;
use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::any::Any;
use core::cmp::min;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use rcore_fs::vfs::FsError::Again;
use rcore_fs::vfs::*;

#[derive(Clone, PartialEq)]
pub enum PipeEnd {
    Read,
    Write,
}

pub struct PipeData {
    buf: VecDeque<u8>,
    eventbus: EventBus,
    /// number of pipe ends
    end_cnt: i32,
    readers: i32,
    writers: i32,
}

pub struct Pipe {
    data: Arc<Mutex<PipeData>>,
    direction: PipeEnd,
}

impl Clone for Pipe {
    fn clone(&self) -> Self {
        let mut data = self.data.lock();
        data.end_cnt += 1;
        match self.direction {
            PipeEnd::Read => data.readers += 1,
            PipeEnd::Write => data.writers += 1,
        }
        drop(data);
        Pipe {
            data: self.data.clone(),
            direction: self.direction.clone(),
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        // pipe end closed
        let mut data = self.data.lock();
        data.end_cnt = data.end_cnt.saturating_sub(1);
        match self.direction {
            PipeEnd::Read => data.readers = data.readers.saturating_sub(1),
            PipeEnd::Write => data.writers = data.writers.saturating_sub(1),
        }
        if data.readers == 0 || data.writers == 0 {
            data.eventbus.set(Event::CLOSED);
        }
    }
}

impl Pipe {
    /// Create a pair of INode: (read, write)
    pub fn create_pair() -> (Pipe, Pipe) {
        let inner = PipeData {
            buf: VecDeque::new(),
            eventbus: EventBus::default(),
            end_cnt: 2, // one read, one write
            readers: 1,
            writers: 1,
        };
        let data = Arc::new(Mutex::new(inner));
        (
            Pipe {
                data: data.clone(),
                direction: PipeEnd::Read,
            },
            Pipe {
                data: data.clone(),
                direction: PipeEnd::Write,
            },
        )
    }

    fn can_read(&self) -> bool {
        if let PipeEnd::Read = self.direction {
            // true
            let data = self.data.lock();
            data.buf.len() > 0 || data.writers == 0
        } else {
            false
        }
    }

    fn can_write(&self) -> bool {
        if let PipeEnd::Write = self.direction {
            self.data.lock().readers > 0
        } else {
            false
        }
    }

    pub fn is_write_closed(&self) -> bool {
        if let PipeEnd::Write = self.direction {
            self.data.lock().readers == 0
        } else {
            false
        }
    }
}

impl INode for Pipe {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if let PipeEnd::Read = self.direction {
            let mut data = self.data.lock();
            if data.buf.len() == 0 && data.writers > 0 {
                Err(Again)
            } else {
                let len = min(buf.len(), data.buf.len());
                for i in 0..len {
                    buf[i] = data.buf.pop_front().unwrap();
                }
                if data.buf.len() == 0 {
                    data.eventbus.clear(Event::READABLE);
                }
                Ok(len)
            }
        } else {
            Ok(0)
        }
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        if let PipeEnd::Write = self.direction {
            let mut data = self.data.lock();
            if data.readers == 0 {
                return Err(FsError::InvalidParam);
            }
            for c in buf {
                data.buf.push_back(*c);
            }
            if !buf.is_empty() {
                data.eventbus.set(Event::READABLE);
            }
            Ok(buf.len())
        } else {
            Ok(0)
        }
    }

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: self.can_read(),
            write: self.can_write(),
            error: self.is_write_closed(),
        })
    }

    fn async_poll<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<PollStatus>> + Send + Sync + 'a>> {
        #[must_use = "future does nothing unless polled/`await`-ed"]
        struct PipeFuture<'a> {
            pipe: &'a Pipe,
        };

        impl<'a> Future for PipeFuture<'a> {
            type Output = Result<PollStatus>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let status = self.pipe.poll()?;
                if status.read || status.write || status.error {
                    return Poll::Ready(Ok(status));
                }
                let waker = cx.waker().clone();
                let mut data = self.pipe.data.lock();
                data.eventbus.subscribe(Box::new({
                    move |_| {
                        waker.wake_by_ref();
                        true
                    }
                }));
                Poll::Pending
            }
        }

        Box::pin(PipeFuture { pipe: self })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
