use crate::error::Result;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::Payment;

pub fn identify(pay1: Payment, pay2: Payment) -> Result<PublicKeyUser> {
    // TODO: We should include here the check for S and payInfo
    let pkUser = (pay2.T * pay1.R - pay1.T * pay2.R) * ((pay1.R - pay2.R).invert().unwrap());
    Ok(PublicKeyUser { pk: pkUser })
}
