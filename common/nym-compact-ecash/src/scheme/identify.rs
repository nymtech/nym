use crate::error::Result;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::Payment;

pub fn identify(pay1: Payment, pay2: Payment) -> Result<PublicKeyUser> {
    // TODO: We should include here the check for S and payInfo
    let pk_user = (pay2.tt * pay1.rr - pay1.tt * pay2.rr) * ((pay1.rr - pay2.rr).invert().unwrap());
    Ok(PublicKeyUser { pk: pk_user })
}
