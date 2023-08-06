// 0       1       2       3       4       5       6       7
// +--------------------------------------------------------------+
// |                       Length (64)                            |
// +--------------------------------------------------------------+
// |                       ID (64)                                |
// +-------+------------------------------------------------------+
// |Type(8)| Payload ...                                          |
// +-------+------------------------------------------------------+

use anyhow::bail;

#[derive(Copy, Clone, Debug)]
pub enum MessageType {
    Connect,
    ConnectAck,
    AudioFrame,
    VideoFrame,
    Unknown,
}

impl From<u8> for MessageType {
    fn from(i: u8) -> Self {
        match i {
            0x0 => MessageType::Connect,
            0x1 => MessageType::ConnectAck,
            0xD => MessageType::VideoFrame,
            0x14 => MessageType::AudioFrame,
            _ => MessageType::Unknown,
        }
    }
}

impl From<MessageType> for u8 {
    fn from(t: MessageType) -> Self {
        match t {
            MessageType::Connect => 0x0,
            MessageType::ConnectAck => 0x1,
            MessageType::AudioFrame => 0x14,
            MessageType::VideoFrame => 0xD,
            MessageType::Unknown => u8::MAX,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RushHeader {
    pub length: u64,
    pub id: u64,
    pub message_type: MessageType,
}

#[derive(Debug)]
pub struct ConnectMessage {
    pub header: RushHeader,
}

#[derive(Debug)]
pub struct AudioFrameMessage {
    pub header: RushHeader,
}

#[derive(Debug)]
pub struct VideoFrameMessage {
    pub header: RushHeader,
}

#[derive(Debug)]
pub enum RushMessages {
    Connect(ConnectMessage),
    AudioFrame(AudioFrameMessage),
    VideoFrame(VideoFrameMessage),
}

pub fn parse(data: &[u8]) -> anyhow::Result<(RushMessages, usize)> {
    let _last_byte_index = data.len() - 1;
    let data_length = data.len() as u64;

    if data_length < 20 {
        bail!("data too short {:?}", data_length)
    }
    
    let length: u64 = u64::from_be_bytes(data[0..8].try_into().unwrap());

    if data_length < length {
        bail!("data too short {:?} {:?}", data_length, length)
    }

    let id: u64 = u64::from_be_bytes(data[8..16].try_into().unwrap());
    let message_type: MessageType = data[16].into();
    let header = RushHeader {
        length,
        id,
        message_type,
    };
    match message_type {
        MessageType::Connect => {
            Ok((RushMessages::Connect(ConnectMessage {
                header,
            }), length.try_into().unwrap()))
        }
        MessageType::ConnectAck => todo!(),
        MessageType::AudioFrame => {
            Ok((RushMessages::AudioFrame(AudioFrameMessage {
                header,
            }), length.try_into().unwrap()))
        }
        MessageType::VideoFrame => {
            Ok((RushMessages::VideoFrame(VideoFrameMessage {
                header,
            }), length.try_into().unwrap()))
        }
        MessageType::Unknown => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn parse_test() -> anyhow::Result<()> {
        let data: Vec<u8> = vec![0x04, 0x02, 0x03, 0x08, 0x02, 0x28, 0x01, 0x03];
        let _ = parse(&data);

        Ok(())
    }
}