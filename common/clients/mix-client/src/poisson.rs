use rand_distr::{Distribution, Exp};

pub fn sample(average_delay: f64) -> f64 {
    // this is our internal code used by our traffic streams
    // the error is only thrown if average delay is less than 0, which will never happen
    // so call to unwrap is perfectly safe here
    let exp = Exp::new(1.0 / average_delay).unwrap();
    exp.sample(&mut rand::thread_rng())
}
