use rocket::State;

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use rocket_okapi::openapi;

#[openapi(tag = "circulating-supply")]
#[get("/current")]
pub(crate) async fn get_circulating_supply(cache: &State<CirculatingSupplyCache>) -> String {
    cache.say_foomp().await.unwrap().value
}
