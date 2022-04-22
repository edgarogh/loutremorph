//! Background-threaded GIF-encoding utilities

use image::codecs::gif::{GifEncoder as ImgGifEncoder, Repeat};
use image::{Frame, ImageResult, RgbaImage};
use indicatif::{ProgressBar, ProgressIterator};
use std::sync::mpsc;
use std::thread::JoinHandle;

pub struct GifEncoder {
    sender: mpsc::Sender<Frame>,
    join_handle: JoinHandle<ImageResult<()>>,
}

impl GifEncoder {
    pub fn new(writer: impl std::io::Write + Send + 'static, progress: ProgressBar) -> Self {
        let (sender, receiver) = mpsc::channel();

        let mut gif = ImgGifEncoder::new(writer);
        gif.set_repeat(Repeat::Infinite).expect("cannot set repeat");

        let join_handle =
            std::thread::spawn(move || gif.encode_frames(receiver.iter().progress_with(progress)));

        Self {
            sender,
            join_handle,
        }
    }

    pub fn write_frame(&self, image: RgbaImage, delay: image::Delay) {
        self.sender
            .send(Frame::from_parts(image, 0, 0, delay))
            .unwrap();
    }

    pub fn flush(self) -> ImageResult<()> {
        std::mem::drop(self.sender);
        self.join_handle.join().unwrap()
    }
}
