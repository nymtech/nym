use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_controllers::Admin;

use crate::{constants::ADMIN_KEY, Result};

const ADMIN: Admin = Admin::new(ADMIN_KEY);

pub(crate) fn set_admin(deps: DepsMut<'_>, admin: Addr) -> Result<()> {
    Ok(ADMIN.set(deps, Some(admin))?)
}

pub(crate) fn assert_admin(deps: Deps, sender: &Addr) -> Result<()> {
    Ok(ADMIN.assert_admin(deps, sender)?)
}
