pub enum Arg {
    // Core OSC Type Tags
    Int(i32),
    Float(f32),
    Str(String),
    Blob(Vec<u8>),
    // Nonstandard OSC Type Tags
    Int64(i64),
    Double(f64),
}

fn arg_char_repr(arg: &Arg) -> char {
    use self::Arg::*;
    match arg {
        Int(_) => 'i',
        Float(_) => 'f',
        Str(_) => 's',
        Blob(_) => 'b',
        Int64(_) => 'h',
        Double(_) => 'd',
    }
}

fn write_string(arg: String) -> Vec<u8> {
    let mut bytes = arg.as_bytes().to_vec();
    bytes.append(&mut vec![b'\0'; 4 - (arg.len() % 4)]);
    assert!(bytes.len() % 4 == 0);
    bytes
}

fn write_blob(mut arg: Vec<u8>) -> Vec<u8> {
    let mut size_bytes: Vec<u8> = (arg.len() as i32).to_be_bytes().to_vec();
    arg.append(&mut vec![b'\0'; 4 - (arg.len() % 4)]);
    assert!(arg.len() % 4 == 0);
    size_bytes.append(&mut arg);
    size_bytes
}

fn write_arg(arg: &Arg) -> Vec<u8> {
    use self::Arg::*;
    match arg {
        Float(f) => f.to_be_bytes().to_vec(),
        Double(d) => d.to_be_bytes().to_vec(),
        Int(i) => i.to_be_bytes().to_vec(),
        Int64(h) => h.to_be_bytes().to_vec(),
        Str(s) => write_string(s.to_string()),
        Blob(b) => write_blob(b.to_vec()),
    }
}

pub struct OscMessage<'a> {
    address: &'a str,
    args: Vec<Arg>,
}

impl<'a> OscMessage<'a> {
    pub fn new(address: &'a str, args: Vec<Arg>) -> Self {
        Self { address, args }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut msg: Vec<u8> = Vec::new();

        msg.append(&mut write_string(self.address.to_string()));

        let mut message_arg_types = ",".to_string();

        if self.args.is_empty() {
            message_arg_types = String::from_utf8(write_string(
                String::from_utf8(message_arg_types.into_bytes()).unwrap(),
            ))
            .unwrap();
            msg.append(&mut message_arg_types.into_bytes());
            return msg;
        }

        let mut message_arguments = Vec::new();

        for arg in &self.args {
            message_arg_types.push(arg_char_repr(arg));
            message_arguments.append(&mut write_arg(arg));
        }

        msg.append(&mut write_string(message_arg_types));
        msg.append(&mut message_arguments);

        msg
    }
}
