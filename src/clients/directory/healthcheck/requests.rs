#[cfg(not(test))]
use mockito;

struct Requester {
    base_url: String
}

trait HealthCheckRequester {
    fn new(base_url: String) -> Self;
    fn get(&self) -> bool;
}

impl HealthCheckRequester for Requester {
    fn new(base_url: String) -> Requester {
        Requester {
            base_url,
        }
    }

    fn get(&self) -> bool {
        let url =  format!("{}/healthcheck", self.base_url);
        match reqwest::get(&url)  {
            Ok(response) => {
                 if response.status() == 200 {
                     true
                 } else {
                     false
                 }
            },
            Err(e) => false,
        }
    }
}

struct HealthCheckResponse {
    ok: bool,
}

mod healthcheck_requests {
    use super::*;
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[test]
        fn it_returns_false() {
            let _m = mock("GET", "/healthcheck")
                .with_status(400)
                .create();
            let req = Requester::new(mockito::server_url());

            let expected = false;
            assert_eq!(expected, req.get());
        }
    }

    #[cfg(test)]
    mod on_a_200_with_ok_json {
        use super::*;

        #[test]
        fn it_returns_true() {
            let _m = mock("GET", "/healthcheck")
                .with_status(200)
                .create();
            let req = Requester::new(mockito::server_url());

            let expected = true;
            assert_eq!(expected, req.get());
        }
    }
}


#[cfg(test)]
mod fixtures {

}