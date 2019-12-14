use std::net::SocketAddr;
use ws::{listen, CloseCode, Handler, Message, Result, Sender};

struct Server {
    out: Sender,
}

impl Handler for Server {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        foomp();
        // Echo the message back
        self.out.send(msg)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("The client is done with the connection."),
            CloseCode::Away => println!("The client is leaving the site."),
            _ => println!("The client encountered an error: {}", reason),
        }
    }
}

pub fn start(socket_address: SocketAddr) {
    listen(socket_address, |out| Server { out: out }).unwrap()
}

fn foomp() {
    println!("FOOOOOOOOOOOOOOOOOOOOOOOOMMMMMMMMMMMMPPPPPPPPPPPPPPPPP");
}
