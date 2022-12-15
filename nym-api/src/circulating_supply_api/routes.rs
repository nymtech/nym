use rocket::State;

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use rocket_okapi::openapi;

#[openapi(tag = "circulating-supply")]
#[get("/current")]
pub(crate) fn get_circulating_supply(cache: &State<CirculatingSupplyCache>) -> &'static str {
    cache.say_foomp()
}
