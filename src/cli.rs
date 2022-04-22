//! [clap] deserialization type definitions

use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(clap::Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Output path [default: <IMG_HUMAN without extension>.gif]
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Duration of a single GIF frame
    #[clap(short = 'd', long, default_value_t = 50)]
    pub frame_duration: u32,

    /// Number of interpolated frames
    #[clap(short = 'i', long, default_value_t = 30)]
    pub interp_len: usize,

    /// Number of static frames between interpolations
    #[clap(short = 'p', long, default_value_t = 15)]
    pub pause_len: u32,

    /// Open the GIF file after it has been generated
    #[cfg(feature = "opener")]
    #[clap(name = "open", long)]
    pub open_gif_after: bool,

    /// Path to an image representing a human face
    pub img_human: PathBuf,

    /// Path to an image representing an otter
    pub img_otter: PathBuf,

    /// Path to a 4-column CSV file containing interpolated points
    ///
    /// The first 2 columns are (x, y) coordinates of a point on the human face file.
    /// The last 2 are (x, y) coordinates of the corresponding point on the otter.
    pub points_csv: PathBuf,
}

impl Cli {
    pub fn pause_duration(&self) -> image::Delay {
        image::Delay::from_numer_denom_ms(self.frame_duration * self.pause_len, 1)
    }

    pub fn frame_duration(&self) -> image::Delay {
        image::Delay::from_numer_denom_ms(self.frame_duration, 1)
    }

    fn name(&self) -> Cow<str> {
        self.img_human.file_stem().unwrap().to_string_lossy()
    }

    pub fn output(&self) -> Cow<OsStr> {
        match &self.output {
            Some(path) => Cow::Borrowed(path.as_os_str()),
            None => Cow::Owned(format!("{}.gif", self.name()).into()),
        }
    }
}
