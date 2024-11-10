use protocol_handler::ProtocolHandler;

fn main() {
    let ph: ProtocolHandler = ProtocolHandler {
        name: "asd".to_string(),
        protocol_name: "broski".to_string(),
    };
    ph.register().unwrap();
}
