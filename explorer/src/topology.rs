use reqwest::Error;
use std::io::prelude::*;
use std::{fs::File, path::Path, time::Duration};
use tokio::time;

pub async fn renew_periodically() -> Result<(), Error> {
    let mut interval_day = time::interval(Duration::from_secs(5));
    loop {
        interval_day.tick().await;
        let topology_json =
            reqwest::get("http://qa-validator.nymtech.net:8081/api/mixmining/topology")
                .await?
                .text()
                .await?;
        save(topology_json)
    }
}

fn save(text: String) {
    let path = Path::new("static/topology.json");
    let display = path.display();

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    match file.write_all(text.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", display, why),
        Ok(_) => (),
    }
}
