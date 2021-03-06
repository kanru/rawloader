use std;
extern crate rayon;
use self::rayon::prelude::*;

pub mod demosaic;
pub mod level;
pub mod colorspaces;
pub mod curves;
pub mod gamma;

use decoders::Image;

extern crate time;

#[derive(Debug, Clone)]
pub struct OpBuffer {
  pub width: usize,
  pub height: usize,
  pub colors: usize,
  pub data: Vec<f32>,
}

impl OpBuffer {
  pub fn new(width: usize, height: usize, colors: usize) -> OpBuffer {
    OpBuffer {
      width: width,
      height: height,
      colors: colors,
      data: vec![0.0; width*height*(colors as usize)],
    }
  }

  pub fn mutate_lines<F>(&mut self, closure: &F)
    where F : Fn(&mut [f32], usize)+std::marker::Sync {

    self.data.par_chunks_mut(self.width*self.colors).enumerate().for_each(|(row, line)| {
      closure(line, row);
    });
  }

  pub fn process_into_new<F>(&self, colors: usize, closure: &F) -> OpBuffer
    where F : Fn(&mut [f32], &[f32])+std::marker::Sync {

    let mut out = OpBuffer::new(self.width, self.height, colors);
    out.data.par_chunks_mut(out.width*out.colors).enumerate().for_each(|(row, line)| {
      closure(line, &self.data[self.width*self.colors*row..]);
    });
    out
  }
}

fn do_timing<O, F: FnMut() -> O>(name: &str, mut closure: F) -> O {
  let from_time = time::precise_time_ns();
  let ret = closure();
  let to_time = time::precise_time_ns();
  println!("{} ms for '{}'", (to_time - from_time)/1000000, name);

  ret
}

pub fn simple_decode (img: &Image, maxwidth: usize, maxheight: usize) -> OpBuffer {
  // Demosaic into 4 channel f32 (RGB or RGBE)
  let mut channel4 = do_timing("demosaic", ||demosaic::demosaic_and_scale(img, maxwidth, maxheight));

  do_timing("level_and_balance", || { level::level_and_balance(img, &mut channel4) });
  // From now on we are in 3 channel f32 (RGB or Lab)
  let mut channel3 = do_timing("camera_to_lab", ||colorspaces::camera_to_lab(img, &channel4));
  do_timing("base_curve", ||curves::base(img, &mut channel3));
  do_timing("lab_to_rec709", ||colorspaces::lab_to_rec709(img, &mut channel3));
  do_timing("gamma", ||gamma::gamma(img, &mut channel3));

  channel3
}
