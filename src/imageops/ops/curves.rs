use std::cmp;
use decoders::RawImage;
use imageops::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpBaseCurve {
  points: Vec<(f32, f32)>,
}

impl OpBaseCurve {
  pub fn new(_img: &RawImage) -> OpBaseCurve {
    OpBaseCurve{
      points: vec![
        (0.00, 0.00),
        (0.30, 0.25),
        (0.50, 0.50),
        (0.70, 0.75),
        (1.00, 1.00),
      ],
    }
  }
}

impl<'a> ImageOp<'a> for OpBaseCurve {
  fn name(&self) -> &str {"basecurve"}
  fn run(&self, pipeline: &mut PipelineGlobals, inid: BufHash, outid: BufHash) {
    let mut buf = (*pipeline.cache.get(&inid).unwrap()).clone();
    let func = SplineFunc::new(self.points.clone());

    buf.mutate_lines(&(|line: &mut [f32], _| {
      for pix in line.chunks_mut(3) {
        pix[0] = func.interpolate(pix[0]);
        pix[1] = pix[1];
        pix[2] = pix[2];
      }
    }));

    pipeline.cache.put(outid, buf, 1);
  }
}


struct SplineFunc {
  points: Vec<(f32,f32)>,
  c1s: Vec<f32>,
  c2s: Vec<f32>,
  c3s: Vec<f32>,
}

impl SplineFunc {
  // Monotone cubic interpolation code adapted from the Javascript example in Wikipedia
  fn new(points: Vec<(f32,f32)>) -> SplineFunc {
    if points.len() < 2 { panic!("Need at least 2 points for Spline"); }

    // Get consecutive differences and slopes
    let mut dxs = Vec::new();
    let mut dys = Vec::new();
    let mut slopes = Vec::new();
    for i in 0..(points.len()-1) {
      let dx = points[i+1].0 - points[i].0;
      let dy = points[i+1].1 - points[i].1;
      dxs.push(dx);
      dys.push(dy);
      slopes.push(dy/dx);
    }
    
    // Get degree-1 coefficients
    let mut c1s = vec![slopes[0]];
    for i in 0..(dxs.len()-1) {
      let m = slopes[i];
      let next = slopes[i+1];
      if m*next <= 0.0 {
        c1s.push(0.0);
      } else {
        let dx = dxs[i];
        let dxnext = dxs[i+1];
        let common = dx + dxnext;
        c1s.push(3.0*common/((common+dxnext)/m + (common + dx)/next));
      }
    }
    c1s.push(slopes[slopes.len()-1]);
  
    // Get degree-2 and degree-3 coefficients
    let mut c2s = Vec::new();
    let mut c3s = Vec::new();
    for i in 0..(c1s.len()-1) {
      let c1 = c1s[i];
      let slope = slopes[i];
      let invdx = 1.0 / dxs[i];
      let common = c1+c1s[i+1] - slope - slope;
      c2s.push((slope-c1-common)*invdx);
      c3s.push(common*invdx*invdx);
    }
  
    SplineFunc {
      points: points,
      c1s: c1s,
      c2s: c2s,
      c3s: c3s,
    }
  }

  fn interpolate(&self, val: f32) -> f32 {
    // Anything at or beyond the last value returns the last value
    let end = self.points[self.points.len()-1].0;
    if val >= end {
      return self.points[self.points.len()-1].1;
    }
    
    // Search for the interval x is in, returning the corresponding y if x is one of the original xs
    let mut low: isize = 0;
    let mut mid: isize;
    let mut high: isize = (self.c3s.len() - 1) as isize;

    while low <= high {
      mid = (low+high)/2;
      let xhere = self.points[mid as usize].0;
      if xhere < val { low = mid + 1; }
      else if xhere > val { high = mid - 1; }
      else { return self.points[mid as usize].1 }
    }
    let i = cmp::max(0, high) as usize;
    
    // Interpolate
    let diff = val - self.points[i].0;

    self.points[i].1 + self.c1s[i]*diff + self.c2s[i]*diff*diff + self.c3s[i]*diff*diff*diff
  }
}
