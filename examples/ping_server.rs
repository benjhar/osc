use osc::{errors::Error, server::OscServer, Arg, OscMessage};

fn ping(_: &OscMessage) -> Option<Vec<Arg>> {
    Some(vec![Arg::Str("Pong!".to_string())])
}

fn main() -> Result<(), Error> {
    let server = OscServer::new("0.0.0.0:0", 1024)?.add_route("/ping", ping);

    server.start().expect("Server crashed");

    Ok(())
}
