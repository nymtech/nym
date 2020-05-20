// Basically to follow identical structure to sphinx::error or std::io::error, etc.
pub struct CoconutError;

pub type Result<T> = std::result::Result<T, CoconutError>;
