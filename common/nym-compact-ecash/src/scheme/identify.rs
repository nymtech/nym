use std::convert::TryFrom;

use crate::error::Result;
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::Parameters;
use crate::scheme::{PayInfo, Payment};

pub fn identify(
    params: &Parameters,
    pay1: Payment,
    pay2: Payment,
    payInfo1: PayInfo,
    payInfo2: PayInfo,
) -> Result<PublicKeyUser> {
    // TODO: We had to include checks for S1, S2 and payinfo1 and payinfo2
    let pkUser = (pay2.T * pay1.R - pay1.T * pay2.R) * (pay1.R - pay2.R).invert().unwrap();
    Ok(PublicKeyUser { pk: pkUser })
}
