mod cli;

use clap::Parser;
use cli::Cli;
use image::buffer::ConvertBuffer;
use image::codecs::gif::{GifEncoder, Repeat};
use image::imageops::FilterType;
use image::{Frame, RgbaImage};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use interpolation::Lerp;
use rayon::prelude::*;

lazy_static::lazy_static! {
    static ref PROGRESS_STYLE: ProgressStyle = {
        ProgressStyle::with_template(
            "[{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
    };
}

/// Create a progress bar quickly
fn pb(count: usize, message: &'static str) -> ProgressBar {
    ProgressBar::new(count as u64)
        .with_message(message)
        .with_style(PROGRESS_STYLE.clone())
}

fn change_alpha(img: &mut RgbaImage, opacity: f32) {
    debug_assert!(opacity >= 0.0);
    debug_assert!(opacity <= 1.0);

    img.par_chunks_mut(4).for_each(|chunk| match chunk {
        [_, _, _, 0] => (),
        [_, _, _, a] => *a = 0.lerp(a, &opacity),
        _ => (),
    });
}

fn main() {
    let cli = Cli::parse();
    let output_path = cli.output();

    let from = image::open(&cli.img_human).unwrap().into_rgb8();
    let to = image::open(&cli.img_otter).unwrap().into_rgb8();

    assert_eq!(from.dimensions(), (2048, 2048));
    assert_eq!(to.dimensions(), (2048, 2048));

    let points_reader = csv::Reader::from_path(&cli.points_csv).unwrap();

    let (human_points, otter_points) = points_reader
        .into_records()
        .enumerate()
        .filter_map(|(idx, record)| {
            let record = record.unwrap();
            if idx == 0 && record.iter().eq(["hx", "hy", "ox", "oy"]) {
                None
            } else {
                Some(record)
            }
        })
        .map(|record| {
            assert_eq!(record.len(), 4);

            let mut floats = record.iter().map(|str| str.trim()).map(|s| {
                s.parse::<f32>()
                    .map_err(|e| e.to_string())
                    .or_else(|_| {
                        s.parse::<u32>()
                            .map(|n| n as f32)
                            .map_err(|e| e.to_string())
                    })
                    .expect(s)
            });

            (
                (
                    floats.next().unwrap_or_default(),
                    floats.next().unwrap_or_default(),
                ),
                (
                    floats.next().unwrap_or_default(),
                    floats.next().unwrap_or_default(),
                ),
            )
        })
        .unzip::<_, _, Vec<(f32, f32)>, Vec<(f32, f32)>>();

    let ratios = (0..cli.interp_len)
        .map(|step| step as f32 / (cli.interp_len - 1) as f32)
        .collect::<Vec<_>>();

    let steps = ratios
        .iter()
        .map(|ratio| {
            human_points
                .iter()
                .zip(&otter_points)
                .map(|(a, b)| (a.0.lerp(&b.0, ratio), a.1.lerp(&b.1, ratio)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let frames = steps
        .iter()
        .zip(ratios)
        .map(|(dst, ratio)| {
            let mut warped = moving_least_squares_image::reverse_dense(
                &from,
                &human_points,
                dst,
                moving_least_squares::deform_affine,
            )
            .convert();

            let mut warped_reverse = moving_least_squares_image::reverse_dense(
                &to,
                &otter_points,
                dst,
                moving_least_squares::deform_affine,
            )
            .convert();

            change_alpha(&mut warped, 1.0 - ratio);
            image::imageops::overlay(&mut warped_reverse, &warped, 0, 0);

            image::imageops::resize(&warped_reverse, 256, 256, FilterType::Nearest)
        })
        .progress_with(pb(cli.interp_len, "computing interpolation"))
        .collect::<Vec<_>>();

    // Compo
    let mut gif = GifEncoder::new(std::fs::File::create(&output_path).unwrap());
    gif.set_repeat(Repeat::Infinite).unwrap();

    let compo = std::iter::repeat(&frames[0])
        .take(cli.pause_len)
        .chain(frames.iter())
        .chain(std::iter::repeat(frames.last().unwrap()).take(cli.pause_len))
        .chain(frames.iter().rev())
        .collect::<Vec<_>>();

    let compo_len = compo.len();
    for frame in compo
        .into_iter()
        .progress_with(pb(compo_len, "encoding gif"))
    {
        gif.encode_frame(Frame::from_parts(frame.clone(), 0, 0, cli.frame_duration()))
            .unwrap()
    }

    #[cfg(feature = "opener")]
    if cli.open_gif_after {
        opener::open(output_path).unwrap();
    }
}
