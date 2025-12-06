//! Easing functions for spinner animations
//!
//! Based on the loading_spinners example from iced by nicksenger:
//! https://github.com/iced-rs/iced/tree/master/examples/loading_spinners
//!
//! MIT License
//!
//! Copyright 2019 Héctor Ramón, Iced contributors
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use iced::Point;

use lyon_algorithms::measure::PathMeasurements;
use lyon_algorithms::path::{Path, builder::NoAttributes, path::BuilderImpl};

use std::sync::LazyLock;

pub static STANDARD: LazyLock<Easing> = LazyLock::new(|| {
    Easing::builder()
        .cubic_bezier_to([0.2, 0.0], [0.0, 1.0], [1.0, 1.0])
        .build()
});

pub struct Easing {
    path: Path,
    measurements: PathMeasurements,
}

impl Easing {
    pub fn builder() -> Builder {
        Builder::new()
    }

    pub fn y_at_x(&self, x: f32) -> f32 {
        let mut sampler = self
            .measurements
            .create_sampler(&self.path, lyon_algorithms::measure::SampleType::Normalized);
        let sample = sampler.sample(x);

        sample.position().y
    }
}

pub struct Builder(NoAttributes<BuilderImpl>);

impl Builder {
    pub fn new() -> Self {
        let mut builder = Path::builder();
        builder.begin(lyon_algorithms::geom::point(0.0, 0.0));

        Self(builder)
    }

    /// Adds a cubic bezier curve. Points must be between 0,0 and 1,1
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

        Easing { path, measurements }
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
