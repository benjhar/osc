pub mod client;
pub mod errors;

use errors::Error;

#[derive(Debug, Clone)]
pub enum Arg {
    // Core OSC Type Tags
    Int(i32),
    Float(f32),
    Str(String),
    Blob(Vec<u8>),
}

fn arg_char_repr(arg: &Arg) -> char {
    use self::Arg::*;
    match arg {
        Int(_) => 'i',
        Float(_) => 'f',
        Str(_) => 's',
        Blob(_) => 'b',
    }
}

fn type_tag_to_default_arg(tag: char) -> Result<Arg, Error> {
    match tag {
        'i' => Ok(Arg::Int(0)),
        'f' => Ok(Arg::Float(0.0)),
        's' => Ok(Arg::Str("".to_string())),
        'b' => Ok(Arg::Blob(Vec::new())),
        _ => Err(Error::UnrecognisedTypeTag(tag)),
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
        // Double(d) => d.to_be_bytes().to_vec(),
        Int(i) => i.to_be_bytes().to_vec(),
        // Int64(h) => h.to_be_bytes().to_vec(),
        Str(s) => write_string(s.to_string()),
        Blob(b) => write_blob(b.to_vec()),
    }
}

fn scan_into_byte_array(arr: &mut [u8], idx: &mut usize, data: &[u8]) -> Result<(), Error> {
    let length = arr.len();
    for item in &mut *arr {
        *item = *data
            .get(*idx)
            .ok_or_else(|| Error::DataLength(length, *idx))?;
        *idx += 1;
    }
    Ok(())
}

#[derive(Clone)]
pub struct OscMessage {
    pub address: String,
    pub args: Vec<Arg>,
}

impl OscMessage {
    pub fn new(address: impl ToString, args: Vec<Arg>) -> Self {
        Self {
            address: address.to_string(),
            args,
        }
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

    pub fn parse_bytes(data: &[u8]) -> Result<Self, Error> {
        if data.len() % 4 != 0 {
            // All valid OSC data has a length multiple of 32, so error if not.
            return Err(Error::Alignment(data.len(), 4));
        }

        let mut curr_datagram = Vec::new();
        let mut i: usize = 0;

        while i < data.len() {
            if data[i] != 0 {
                curr_datagram.push(data[i]);
            } else {
                break;
            }
            i += 1;
        }

        let address: String = match String::from_utf8(std::mem::take(&mut curr_datagram)) {
            Ok(s) => s,
            Err(_) => return Err(Error::Utf8("OSC address".to_string())),
        };

        // Skip to the next part, which is always 32bit/4 byte aligned
        i += 4 - (i % 4);

        while i < data.len() {
            if data[i] != 0 {
                curr_datagram.push(data[i]);
            } else {
                break;
            }
            i += 1;
        }

        i += 4 - (i % 4);

        let mut arg_types_str = match String::from_utf8(std::mem::take(&mut curr_datagram)) {
            Ok(s) => s,
            Err(_) => {
                return Err(Error::Utf8("OSC argument type tags".to_string()));
            }
        };

        if !arg_types_str.is_empty() && arg_types_str.remove(0) != ',' {
            return Err(Error::Malformed("OSC argument type tags".to_string()));
        }

        // Prepare args vec by scanning argument types
        let mut args: Vec<Arg> = Vec::new();
        for arg_tag in arg_types_str.chars() {
            args.push(type_tag_to_default_arg(arg_tag)?);
        }

        let mut four_bytes = [0; 4];
        if !args.is_empty() {
            for arg in &mut args {
                use self::Arg::*;
                match arg {
                    Int(_) => {
                        scan_into_byte_array(&mut four_bytes, &mut i, data)?;
                        *arg = Int(i32::from_be_bytes(four_bytes));
                    }
                    Float(_) => {
                        scan_into_byte_array(&mut four_bytes, &mut i, data)?;
                        *arg = Float(f32::from_be_bytes(four_bytes));
                    }
                    Str(_) => {
                        while i < data.len() {
                            if data[i] != 0 {
                                curr_datagram.push(data[i]);
                            } else {
                                break;
                            }
                            i += 1;
                        }
                        i += 4 - (i % 4);
                        match String::from_utf8(std::mem::take(&mut curr_datagram)) {
                            Ok(s) => *arg = Str(s),
                            Err(_) => {
                                return Err(Error::Utf8("OSC string".to_string()));
                            }
                        }
                    }
                    Blob(_) => {
                        scan_into_byte_array(&mut four_bytes, &mut i, data)?;
                        let blob_size = i32::from_be_bytes(four_bytes);
                        let mut blob = vec![0; blob_size as usize];
                        scan_into_byte_array(&mut blob, &mut i, data)?;
                        *arg = Blob(blob);
                    }
                }
            }
        }

        Ok(Self::new(address, args))
    }
}
