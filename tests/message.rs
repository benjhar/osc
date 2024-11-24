use osc::{Arg, OscMessage};

#[test]
fn encode_decode() {
    let message = OscMessage::new("/test", vec![Arg::Str("test".to_string())]);
    let bytes = message.build().expect("Failed to build message");
    let decoded_message = OscMessage::parse_bytes(&bytes).expect("Failed to decode message");
    assert!(decoded_message == message);
}
