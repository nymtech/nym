use rand_distr::{Distribution, Exp};

pub fn sample(average_delay: f64) -> f64 {
    let exp = Exp::new(1.0 / average_delay).unwrap();
    exp.sample(&mut rand::thread_rng())
}
