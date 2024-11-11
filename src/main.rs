use protocol_handler::ProtocolHandler;

fn main() {
    let ph: ProtocolHandler = ProtocolHandler {
        name: "myapp".to_string(),
        protocol_name: "myapp".to_string(),
    };
    ph.register().unwrap();
}
