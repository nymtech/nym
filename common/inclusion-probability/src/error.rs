#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The list of cumulative stake was unexpectedly empty")]
    EmptyListCumulStake,
    #[error("Sample point was unexpectedly out of bounds")]
    SamplePointOutOfBounds,
    #[error("Norm computation failed on different size arrays")]
    NormDifferenceSizeArrays,
    #[error("Computed probabilities are fewer than input number of nodes")]
    ResultsShorterThanInput,
}
