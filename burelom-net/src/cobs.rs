/// Encode data with COBS algorithm
/// 
/// # Arguments
/// `data` - plaintext for encoding
/// 
/// # Returns - encoded data with guaranteed asense of 0x00 bytes except last terminal 
/// byte (always 0x00)
pub fn encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![0x00];
    }

    let mut output = Vec::with_capacity(data.len() * 2);
    let mut code = 1u8;
    let mut code_index = 0usize;
    output.push(0); 

    for &byte in data {
        if byte == 0x00 {
            output[code_index] = code;
            output.push(0);
            code_index = output.len() - 1;
            code = 1;
        } else {
            output.push(byte);
            code += 1;
            if code == 255 {
                output[code_index] = 255;
                output.push(0);
                code_index = output.len() - 1;
                code = 1;
            }
        }
    }

    output[code_index] = code;
    output.push(0x00);

    output
}

/// Decode data encoded by COBS algorithm with 0x00 terminal byte
/// 
/// # Arguments
/// `encoded` - data encoded by COBS
/// 
/// # Returns - plain text or None if encoding currpted
pub fn decode(encoded: &[u8]) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(encoded.len());
    let mut i = 0;

    while i < encoded.len() {
        let code = encoded[i];
        if code == 0 {
            if i == encoded.len() - 1 {
                break;
            } else {
                return None; 
            }
        }

        i += 1;
        let len = (code - 1) as usize;

        for _ in 0..len {
            if i >= encoded.len() {
                return None;
            }
            let b = encoded[i];
            if b == 0x00 {
                return None; 
            }
            output.push(b);
            i += 1;
        }

        if code != 255 && i < encoded.len() && encoded[i] != 0x00 {
            output.push(0x00);
        }
    }

    Some(output)
}