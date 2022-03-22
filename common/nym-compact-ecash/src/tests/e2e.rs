use crate::error::CompactEcashError;
use crate::scheme::keygen::{generate_keypair_user, PublicKeyUser, SecretKeyUser, ttp_keygen};
use crate::scheme::setup::Parameters;
use crate::scheme::withdrawal::{issue_wallet, withdrawal_request};

#[test]
fn main() -> Result<(), CompactEcashError> {
    let params = Parameters::new().unwrap();
    let user_keypair = generate_keypair_user(&params);

    let (req, req_info) = withdrawal_request(&params, &user_keypair.secret_key()).unwrap();
    let mut authorities_keypairs = ttp_keygen(&params, 1, 1).unwrap();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue_wallet(&params, auth_keypair.secret_key(), user_keypair.public_key(), &req);
    }

    Ok(())
}
