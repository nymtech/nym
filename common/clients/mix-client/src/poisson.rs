use rand_distr::{Distribution, Exp};
use std::time;

pub fn sample(average_duration: time::Duration) -> time::Duration {
    // this is our internal code used by our traffic streams
    // the error is only thrown if average delay is less than 0, which will never happen
    // so call to unwrap is perfectly safe here
    let exp = Exp::new(1.0 / average_duration.as_nanos() as f64).unwrap();
    time::Duration::from_nanos(exp.sample(&mut rand::thread_rng()).round() as u64)
}
