use crate::scheme::withdrawal::WithdrawalRequest;
use crate::traits::Bytable;
use crate::utils::BlindedSignature;

macro_rules! impl_clone {
    ($struct:ident) => {
        impl Clone for $struct {
            fn clone(&self) -> Self {
                Self::try_from_byte_slice(&self.to_byte_vec()).unwrap()
            }
        }
    };
}

impl_clone!(WithdrawalRequest);
impl_clone!(BlindedSignature);
