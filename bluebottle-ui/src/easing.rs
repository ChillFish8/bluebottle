//! Just taken from https://github.com/iced-rs/iced/blob/master/examples/loading_spinners/src/easing.rs

use std::sync::LazyLock;

use iced::Point;
use lyon_algorithms::measure::PathMeasurements;
use lyon_algorithms::path::Path;
use lyon_algorithms::path::builder::NoAttributes;
use lyon_algorithms::path::path::BuilderImpl;

pub static EMPHASIZED: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.05, 0.0], [0.133333, 0.06], [0.166666, 0.4])
        .cubic_bezier_to([0.208333, 0.82], [0.25, 1.0], [1.0, 1.0])
        .build()
});

pub static EMPHASIZED_DECELERATE: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.05, 0.7], [0.1, 1.0], [1.0, 1.0])
        .build()
});

pub static EMPHASIZED_ACCELERATE: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.3, 0.0], [0.8, 0.15], [1.0, 1.0])
        .build()
});

pub static STANDARD: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.2, 0.0], [0.0, 1.0], [1.0, 1.0])
        .build()
});

pub static STANDARD_DECELERATE: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.0, 0.0], [0.0, 1.0], [1.0, 1.0])
        .build()
});

pub static STANDARD_ACCELERATE: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.3, 0.0], [1.0, 1.0], [1.0, 1.0])
        .build()
});

/// Number of samples in each easing's precomputed lookup table. A power of
/// two gives a precise enough curve for 100-300ms animations at 60 fps and
/// keeps the table small (1 KiB per easing).
const LUT_SIZE: usize = 256;

pub struct Easing {
    lut: [f32; LUT_SIZE],
}

impl Easing {
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Looks up the eased y for x in [0, 1] via linear interpolation between
    /// two adjacent samples in the precomputed table. Avoids reconstructing a
    /// lyon path sampler on every call, which would otherwise dominate the
    /// hover/press animation hot path.
    pub fn y_at_x(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let max = (LUT_SIZE - 1) as f32;
        let idx_f = x * max;
        let lo = idx_f.floor() as usize;
        let hi = (lo + 1).min(LUT_SIZE - 1);
        let t = idx_f - lo as f32;
        self.lut[lo] * (1.0 - t) + self.lut[hi] * t
    }
}

pub struct Builder(NoAttributes<BuilderImpl>);

impl Builder {
    pub fn new() -> Self {
        let mut builder = Path::builder();
        builder.begin(lyon_algorithms::geom::point(0.0, 0.0));

        Self(builder)
    }

    /// Adds a line segment. Points must be between 0,0 and 1,1
    pub fn line_to(mut self, to: impl Into<Point>) -> Self {
        self.0.line_to(Self::point(to));

        self
    }

    /// Adds a quadratic bézier curve. Points must be between 0,0 and 1,1
    pub fn quadratic_bezier_to(
        mut self,
        ctrl: impl Into<Point>,
        to: impl Into<Point>,
    ) -> Self {
        self.0
            .quadratic_bezier_to(Self::point(ctrl), Self::point(to));

        self
    }

    /// Adds a cubic bézier curve. Points must be between 0,0 and 1,1
    pub fn cubic_bezier_to(
        mut self,
        ctrl1: impl Into<Point>,
        ctrl2: impl Into<Point>,
        to: impl Into<Point>,
    ) -> Self {
        self.0
            .cubic_bezier_to(Self::point(ctrl1), Self::point(ctrl2), Self::point(to));

        self
    }

    pub fn build(mut self) -> Easing {
        self.0.line_to(lyon_algorithms::geom::point(1.0, 1.0));
        self.0.end(false);

        let path = self.0.build();
        let measurements = PathMeasurements::from_path(&path, 0.0);
        let mut sampler = measurements
            .create_sampler(&path, lyon_algorithms::measure::SampleType::Normalized);

        let mut lut = [0.0_f32; LUT_SIZE];
        let max = (LUT_SIZE - 1) as f32;
        for (i, slot) in lut.iter_mut().enumerate() {
            let x = i as f32 / max;
            *slot = sampler.sample(x).position().y;
        }

        Easing { lut }
    }

    fn point(p: impl Into<Point>) -> lyon_algorithms::geom::Point<f32> {
        let p: Point = p.into();
        lyon_algorithms::geom::point(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0))
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
