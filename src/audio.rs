#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
mod web {
    // pub struct StaticAudioSource {
    //     inner: web_sys::AudioBuffer,
    // }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct StreamingAudioSource {
        inner: web_sys::HtmlMediaElement,
    }

    impl StreamingAudioSource {
        pub fn from_element(element: web_sys::HtmlMediaElement) -> Self {
            Self { inner: element }
        }
    }

    pub struct Sink {
        source: StreamingAudioSource,
        _node: web_sys::MediaElementAudioSourceNode,
    }

    pub struct AudioContext {
        ctx: once_cell::sync::Lazy<web_sys::AudioContext>,
    }

    fn ctx_init() -> web_sys::AudioContext {
        web_sys::AudioContext::new().unwrap()
    }

    impl AudioContext {
        pub fn new() -> Self {
            // Will only panic is the specified sample rate isn't supported.
            // Since we don't specify sample rate, this will never fail.
            // https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/AudioContext
            let ctx = once_cell::sync::Lazy::new(ctx_init as _);
            Self { ctx }
        }

        pub fn play_new(&self, source: StreamingAudioSource) -> eyre::Result<Sink> {
            let node = self.ctx.create_media_element_source(&source.inner).unwrap();
            node.connect_with_audio_node(&self.ctx.destination())
                .unwrap();
            let _r = source.inner.play();
            Ok(Sink {
                source,
                _node: node,
            })
        }

        pub fn pause(&self, sink: &Sink) {
            let _r = sink.source.inner.pause();
        }

        pub fn play(&self, sink: &Sink) {
            let _r = sink.source.inner.play();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use cursor::CursorOverShared;

    // pub struct StaticAudioSource<S: rodio::Sample> {
    //     inner: Vec<S>,
    // }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct StreamingAudioSource {
        inner: std::sync::Arc<Vec<u8>>,
    }

    impl StreamingAudioSource {
        pub fn from_data(data: std::sync::Arc<Vec<u8>>) -> Self {
            Self { inner: data }
        }
    }

    pub struct AudioContext {
        sender: std::sync::mpsc::Sender<Command>,
        _thread_handle: std::thread::JoinHandle<eyre::Result<()>>,
        sink_id: std::sync::atomic::AtomicUsize,
        volume: f32,
    }

    enum Command {
        PlayNew(Sink, rodio::Decoder<CursorOverShared<Vec<u8>>>),
        Play(Sink),
        Pause(Sink),
        SetGlobalVolume(f32),
    }

    pub type Sink = usize;

    impl AudioContext {
        pub fn new() -> Self {
            let (sx, rx) = std::sync::mpsc::channel();
            let _thread_handle = std::thread::spawn(move || {
                let mut sinks = std::collections::BTreeMap::new();
                let (_stream, stream_handle) = rodio::OutputStream::try_default()?;

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        Command::PlayNew(id, source) => {
                            let sink = rodio::Sink::try_new(&stream_handle)?;
                            sink.append(source);
                            sinks.insert(id, sink);
                        }
                        Command::Pause(id) => {
                            sinks.get(&id).map(rodio::Sink::pause);
                        }
                        Command::Play(id) => {
                            sinks.get(&id).map(rodio::Sink::play);
                        }
                        Command::SetGlobalVolume(volume) => {
                            for sink in sinks.values() {
                                sink.set_volume(volume);
                            }
                        }
                    }
                }

                Ok(())
            });

            Self {
                sender: sx,
                _thread_handle,
                sink_id: Default::default(),
                volume: 0.0,
            }
        }

        pub fn set_global_volume(&mut self, volume: f32) {
            if self.sender.send(Command::SetGlobalVolume(volume)).is_ok() {
                self.volume = volume;
            }
        }

        pub fn global_volume(&self) -> f32 {
            self.volume
        }

        pub fn play_new(&self, source: StreamingAudioSource) -> eyre::Result<Sink> {
            let data = std::io::Cursor::new(source.inner);
            let data = CursorOverShared::new(data);
            let source = rodio::Decoder::new(data)?;
            let order = std::sync::atomic::Ordering::SeqCst;
            let id = self.sink_id.fetch_add(1, order);
            self.sender.send(Command::PlayNew(id, source))?;
            Ok(id)
        }

        pub fn pause(&self, sink: &Sink) {
            let _r = self.sender.send(Command::Pause(*sink));
        }

        pub fn play(&self, sink: &Sink) {
            let _r = self.sender.send(Command::Play(*sink));
        }

        // pub fn play_looped(&mut self, source: StreamingAudioSource) -> Result<Sink, PlayError> {
        //     let source = rodio::decoder::LoopedDecoder
        // }
    }

    mod cursor {
        use std::io::{BufRead, IoSliceMut, Read, Seek, SeekFrom};

        pub struct CursorOverShared<T> {
            inner: std::io::Cursor<std::sync::Arc<T>>,
        }

        impl<T> CursorOverShared<T> {
            pub fn new(cursor: std::io::Cursor<std::sync::Arc<T>>) -> Self {
                Self { inner: cursor }
            }
        }

        impl<T> Seek for CursorOverShared<T>
        where
            T: AsRef<[u8]>,
        {
            fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
                let (base_pos, offset) = match style {
                    SeekFrom::Start(n) => {
                        self.inner.set_position(n);
                        return Ok(n);
                    }
                    SeekFrom::End(n) => (self.inner.get_ref().as_ref().as_ref().len() as u64, n),
                    SeekFrom::Current(n) => (self.inner.position(), n),
                };
                let new_pos = if offset >= 0 {
                    base_pos.checked_add(offset as u64)
                } else {
                    base_pos.checked_sub((offset.wrapping_neg()) as u64)
                };
                match new_pos {
                    Some(n) => {
                        self.inner.set_position(n);
                        Ok(n)
                    }
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid seek to a negative or overflowing position",
                    )),
                }
            }

            fn stream_position(&mut self) -> std::io::Result<u64> {
                Ok(self.inner.position())
            }
        }

        impl<T> Read for CursorOverShared<T>
        where
            T: AsRef<[u8]>,
        {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = Read::read(&mut self.fill_buf()?, buf)?;
                self.inner.set_position(self.inner.position() + n as u64);
                Ok(n)
            }

            fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
                let mut nread = 0;
                for buf in bufs {
                    let n = self.read(buf)?;
                    nread += n;
                    if n < buf.len() {
                        break;
                    }
                }
                Ok(nread)
            }

            fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
                let n = buf.len();
                Read::read_exact(&mut self.fill_buf()?, buf)?;
                self.inner.set_position(self.inner.position() + n as u64);
                Ok(())
            }
        }

        impl<T> BufRead for CursorOverShared<T>
        where
            T: AsRef<[u8]>,
        {
            fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
                let amt = std::cmp::min(
                    self.inner.position(),
                    self.inner.get_ref().as_ref().as_ref().len() as u64,
                );
                Ok(&self.inner.get_ref().as_ref().as_ref()[(amt as usize)..])
            }
            fn consume(&mut self, amt: usize) {
                self.inner.set_position(self.inner.position() + amt as u64);
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn cursor_test() {
                let data = vec![0u8, 1, 2, 3].into_boxed_slice();
                let shared = std::sync::Arc::new(data);
                let cursor = std::io::Cursor::new(shared.clone());
                let cursor = CursorOverShared::new(cursor);
                let decoder = rodio::Decoder::new(cursor);
            }
        }
    }
}
