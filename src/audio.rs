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

    // impl Clone for StreamingAudioSource {
    //     fn clone(&self) -> Self {
    //         use wasm_bindgen::JsCast;
    //         let src = self.inner.current_src();
    //
    //         let inner = web_sys::window()
    //             .expect("window should exist")
    //             .document()
    //             .expect("document should exist")
    //             .create_element("audio")
    //             .expect("should create audio element")
    //             .dyn_into::<web_sys::HtmlMediaElement>()
    //             .expect("should be an audio element");
    //         inner.set_src(&src);
    //         Self { inner }
    //     }
    // }

    impl StreamingAudioSource {
        pub fn from_element(element: web_sys::HtmlMediaElement) -> Self {
            Self { inner: element }
        }
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Sink {
        source: StreamingAudioSource,
        _node: web_sys::MediaElementAudioSourceNode,
    }

    struct InnerContext {
        ctx: web_sys::AudioContext,
        gain: web_sys::GainNode,
    }

    impl InnerContext {
        fn new() -> Self {
            // Will only panic is the specified sample rate isn't supported.
            // Since we don't specify sample rate, this will never fail.
            // https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/AudioContext
            let ctx = web_sys::AudioContext::new().unwrap();
            let gain = ctx.create_gain().unwrap();
            gain.connect_with_audio_node(&ctx.destination()).unwrap();
            gain.gain().set_value(0.);
            Self { ctx, gain }
        }
    }

    impl Drop for InnerContext {
        fn drop(&mut self) {
            let _r = self.ctx.close();
        }
    }

    pub struct AudioContext {
        ctx: once_cell::sync::Lazy<InnerContext>,
    }

    impl AudioContext {
        pub fn new() -> Self {
            let ctx = once_cell::sync::Lazy::new(InnerContext::new as _);
            Self { ctx }
        }

        pub fn set_global_volume(&mut self, volume: f32) {
            self.ctx.gain.gain().set_value(volume);
        }

        pub fn global_volume(&self) -> f32 {
            self.ctx.gain.gain().value()
        }

        pub fn play_new(&self, source: StreamingAudioSource) -> eyre::Result<Sink> {
            let node = self
                .ctx
                .ctx
                .create_media_element_source(&source.inner)
                .unwrap();
            node.connect_with_audio_node(&self.ctx.gain).unwrap();
            // let _r = source.inner.play();
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

        pub fn stop(&self, sink: &Sink) {
            let _r = sink.source.inner.pause();
            sink.source.inner.set_current_time(0.);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    // pub struct StaticAudioSource<S: rodio::Sample> {
    //     inner: Vec<S>,
    // }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct StreamingAudioSource {
        inner: std::sync::Arc<[u8]>,
    }

    impl StreamingAudioSource {
        pub fn from_data(data: Vec<u8>) -> Self {
            Self { inner: data.into() }
        }
    }

    impl AsRef<[u8]> for StreamingAudioSource {
        fn as_ref(&self) -> &[u8] {
            self.inner.as_ref()
        }
    }

    pub struct AudioContext {
        sender: std::sync::mpsc::Sender<Command>,
        _thread_handle: std::thread::JoinHandle<eyre::Result<()>>,
        sink_id: std::sync::atomic::AtomicUsize,
        volume: f32,
    }

    enum Command {
        PlayNew(usize, StreamingAudioSource),
        Play(usize),
        Pause(usize),
        Stop(usize),
        SetGlobalVolume(f32),
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Sink(usize);

    impl AudioContext {
        pub fn new() -> Self {
            let (sx, rx) = std::sync::mpsc::channel();
            let mut volume = 0.;

            let _thread_handle = std::thread::spawn(move || {
                let mut sinks = std::collections::BTreeMap::new();
                let mut sources = std::collections::BTreeMap::new();
                let (_stream, stream_handle) = rodio::OutputStream::try_default()?;

                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        Command::PlayNew(id, data) => {
                            let cursor = std::io::Cursor::new(data.clone());
                            let source = rodio::Decoder::new(cursor)?;
                            let sink = rodio::Sink::try_new(&stream_handle)?;
                            sink.pause();
                            sink.set_volume(volume);
                            sink.append(source);
                            sinks.insert(id, sink);
                            sources.insert(id, data);
                        }
                        Command::Pause(id) => {
                            sinks.get(&id).map(rodio::Sink::pause);
                        }
                        Command::Play(id) => {
                            if let Some(sink) = sinks.get_mut(&id) {
                                if sink.empty() {
                                    if let Some(data) = sources.get(&id) {
                                        let cursor = std::io::Cursor::new(data.clone());
                                        let source = rodio::Decoder::new(cursor)?;
                                        *sink = rodio::Sink::try_new(&stream_handle)?;
                                        sink.set_volume(volume);
                                        sink.append(source);
                                    }
                                }
                                sink.play();
                            }
                        }
                        Command::Stop(id) => {
                            let sink = sinks.get_mut(&id);
                            let data = sources.get(&id);
                            if let Some((sink, data)) = sink.zip(data) {
                                sink.stop();

                                let cursor = std::io::Cursor::new(data.clone());
                                let source = rodio::Decoder::new(cursor)?;
                                sink.append(source);
                            }
                        }
                        Command::SetGlobalVolume(new_volume) => {
                            volume = new_volume;
                            for sink in sinks.values() {
                                sink.set_volume(new_volume);
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
                volume,
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
            let order = std::sync::atomic::Ordering::SeqCst;
            let id = self.sink_id.fetch_add(1, order);
            self.sender.send(Command::PlayNew(id, source))?;
            Ok(Sink(id))
        }

        pub fn pause(&self, sink: &Sink) {
            let _r = self.sender.send(Command::Pause(sink.0));
        }

        pub fn play(&self, sink: &Sink) {
            let _r = self.sender.send(Command::Play(sink.0));
        }

        pub fn stop(&self, sink: &Sink) {
            let _r = self.sender.send(Command::Stop(sink.0));
        }

        // pub fn play_looped(&mut self, source: StreamingAudioSource) -> Result<Sink, PlayError> {
        //     let source = rodio::decoder::LoopedDecoder
        // }
    }
}
