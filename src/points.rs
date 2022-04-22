//! Read and interpolate point arrays

use interpolation::Lerp;
use std::io::Read;

pub struct Points {
    pub human: Vec<(f32, f32)>,
    pub otter: Vec<(f32, f32)>,
}

impl Points {
    pub fn read(reader: impl Read) -> Self {
        let points_reader = csv::Reader::from_reader(reader);

        let (human, otter) = points_reader
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

        Self { human, otter }
    }

    pub fn interpolate(&self, steps: usize) -> (Vec<f32>, Vec<Vec<(f32, f32)>>) {
        let ratios = (0..steps)
            .map(|step| step as f32 / (steps - 1) as f32)
            .collect::<Vec<_>>();

        let steps = ratios
            .iter()
            .map(|ratio| {
                self.human
                    .iter()
                    .zip(&self.otter)
                    .map(|(a, b)| (a.0.lerp(&b.0, ratio), a.1.lerp(&b.1, ratio)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        (ratios, steps)
    }
}
