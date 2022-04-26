mod cli;
mod encoder;
mod points;

use clap::Parser;
use cli::Cli;
use encoder::GifEncoder;
use image::buffer::ConvertBuffer;
use image::imageops::FilterType;
use image::RgbaImage;
use indicatif::{MultiProgress, ProgressBar, ProgressIterator, ProgressStyle};
use interpolation::Lerp;
use points::Points;
use rayon::prelude::*;
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

lazy_static::lazy_static! {
    static ref PROGRESS_STYLE: ProgressStyle = {
        ProgressStyle::with_template(
            "[{elapsed}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
    };
}

/// Create a progress bar quickly
fn pb(multi_progress: &MultiProgress, count: usize, message: &'static str) -> ProgressBar {
    multi_progress.add(
        ProgressBar::new(count as u64)
            .with_message(message)
            .with_style(PROGRESS_STYLE.clone()),
    )
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
    let (img_human, img_otter, points_csv) = match &cli {
        Cli {
            img_human,
            img_otter: Some(img_otter),
            points_csv: Some(points_csv),
            ..
        } => (
            Cow::Borrowed(Path::new(img_human.as_str())),
            Cow::Borrowed(img_otter.as_path()),
            Cow::Borrowed(points_csv.as_path()),
        ),
        Cli {
            img_human: name,
            img_otter: _,
            points_csv: _,
            ..
        } => (
            Cow::Owned(format!("{name}.png").into()),
            Cow::Owned(format!("{name}_l.png").into()),
            Cow::Owned(format!("{name}.csv").into()),
        ),
    };

    let human_img = image::open(&img_human).unwrap().into_rgb8();
    let otter_img = image::open(&img_otter).unwrap().into_rgb8();

    assert_eq!(human_img.dimensions(), otter_img.dimensions());

    let points = Points::read(BufReader::new(File::open(&points_csv).unwrap()));
    let (ratios, steps) = points.interpolate(cli.interp_len);

    let multi_progress = MultiProgress::new();
    let progress_interpolation = pb(&multi_progress, cli.interp_len, "computing interpolation");
    let progress_encoding = pb(&multi_progress, cli.interp_len * 2 + 2, "encoding gif");

    let gif_encoder = GifEncoder::new(
        BufWriter::new(File::create(&cli.output()).unwrap()),
        progress_encoding,
    );

    gif_encoder.write_frame(
        image::imageops::resize(&human_img, 256, 256, FilterType::Nearest).convert(),
        cli.pause_duration(),
    );

    #[allow(clippy::needless_collect)]
    let frames = steps
        .iter()
        .zip(ratios)
        .map(|(dst, ratio)| {
            let mut warped = moving_least_squares_image::reverse_dense(
                &human_img,
                &points.human,
                dst,
                moving_least_squares::deform_affine,
            )
            .convert();

            let mut warped_reverse = moving_least_squares_image::reverse_dense(
                &otter_img,
                &points.otter,
                dst,
                moving_least_squares::deform_affine,
            )
            .convert();

            change_alpha(&mut warped, 1.0 - ratio);
            image::imageops::overlay(&mut warped_reverse, &warped, 0, 0);

            let frame = image::imageops::resize(&warped_reverse, 256, 256, FilterType::Gaussian);

            gif_encoder.write_frame(frame.clone(), cli.frame_duration());

            frame
        })
        .progress_with(progress_interpolation)
        .collect::<Vec<_>>();

    gif_encoder.write_frame(
        image::imageops::resize(&otter_img, 256, 256, FilterType::Nearest).convert(),
        cli.pause_duration(),
    );

    frames.into_iter().rev().for_each(|img| {
        gif_encoder.write_frame(img, cli.frame_duration());
    });

    gif_encoder.flush().unwrap();

    #[cfg(feature = "opener")]
    if cli.open_gif_after {
        opener::open(output_path).unwrap();
    }
}
