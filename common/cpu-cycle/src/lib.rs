use libc::c_int;

#[link(name = "cpucycles")]
extern "C" {
    fn cpucycles() -> c_int;
}

#[cfg(test)]
mod test {
    use crate::cpucycles;

    #[test]
    fn test_cpucycles() {
        unsafe {
            let cycles = cpucycles();
            println!("{cycles}")
        }
    }
}
