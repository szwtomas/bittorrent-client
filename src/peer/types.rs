use log::*;
use std::io::{Read, Write};
use std::net::TcpStream;

const PSTRLEN: u8 = 19;
const HANDSHAKE_LENGTH: usize = 68;

// Message constants
const MESSAGE_ID_SIZE: usize = 1;
const MESSAGE_LENGTH_SIZE: usize = 4;

#[allow(dead_code)]
pub struct Bitfield(Vec<u8>);

impl Bitfield {
    pub fn new() -> Self {
        Bitfield(vec![])
    }

    pub fn non_empty(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn set_bitfield(&mut self, bitfield: &[u8]) {
        self.0 = bitfield.to_vec();
    }

    #[allow(dead_code)]
    fn has_piece(&self, index: usize) -> bool {
        let byte_index = index / 8;
        let offset = index % 8;
        if byte_index >= self.0.len() {
            return false;
        }
        (self.0[byte_index] >> (7 - offset) & 1) != 0
    }

    #[allow(dead_code)]
    fn set_piece(&mut self, index: usize) {
        let byte_index = index / 8;
        let offset = index % 8;

        if byte_index >= self.0.len() {
            return;
        }
        self.0[byte_index] |= 1 << (7 - offset);
    }
}

#[derive(Debug, PartialEq)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub peer_id: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PeerMessageId {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield,
    Request,
    Piece,
    Cancel,
    Port,
}

impl PeerMessageId {
    fn from_u8(id: u8) -> Result<PeerMessageId, String> {
        match id {
            0 => Ok(PeerMessageId::Choke),
            1 => Ok(PeerMessageId::Unchoke),
            2 => Ok(PeerMessageId::Interested),
            3 => Ok(PeerMessageId::NotInterested),
            4 => Ok(PeerMessageId::Have),
            5 => Ok(PeerMessageId::Bitfield),
            6 => Ok(PeerMessageId::Request),
            7 => Ok(PeerMessageId::Piece),
            8 => Ok(PeerMessageId::Cancel),
            9 => Ok(PeerMessageId::Port),
            _ => Err(format!("Invalid message id: {}", id)),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PeerMessage {
    pub id: PeerMessageId,
    pub length: u32,
    pub payload: Vec<u8>,
}

impl PeerMessage {
    pub fn unchoke() -> PeerMessage {
        const UNCHOKE_MSG_LENGTH: u32 = 1;
        PeerMessage {
            id: PeerMessageId::Unchoke,
            length: UNCHOKE_MSG_LENGTH,
            payload: vec![],
        }
    }
    pub fn interested() -> PeerMessage {
        const INTERESTED_MSG_LENGTH: u32 = 1;
        PeerMessage {
            id: PeerMessageId::Interested,
            length: INTERESTED_MSG_LENGTH,
            payload: vec![],
        }
    }

    // function tan conver a u32 into 4 bytes vector big endian
    fn u32_to_vec_be(num: u32) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes[0] = (num >> 24) as u8;
        bytes[1] = (num >> 16) as u8;
        bytes[2] = (num >> 8) as u8;
        bytes[3] = num as u8;
        bytes
    }

    pub fn request(index: u32, begin: u32, length: u32) -> PeerMessage {
        let mut payload = vec![];
        // write index as 4 bytes to payload
        payload.extend_from_slice(&Self::u32_to_vec_be(index));
        payload.extend_from_slice(&Self::u32_to_vec_be(begin));
        payload.extend_from_slice(&Self::u32_to_vec_be(length));

        PeerMessage {
            id: PeerMessageId::Request,
            length: (payload.len() + 1) as u32,
            payload,
        }
    }
    // TODO: handle error
    pub fn piece(piece_index: u32, offset: u32, block: Vec<u8>) -> PeerMessage {
        let mut payload = vec![];
        payload.extend_from_slice(&Self::u32_to_vec_be(piece_index));
        payload.extend_from_slice(&Self::u32_to_vec_be(offset));
        payload.extend_from_slice(&block);

        PeerMessage {
            id: PeerMessageId::Piece,
            length: (payload.len() + 1) as u32,
            payload,
        }
    }

    pub fn keep_alive() -> PeerMessage {
        PeerMessage {
            id: PeerMessageId::Choke,
            length: 0,
            payload: vec![],
        }
    }
}

pub struct PeerMessageStream {
    stream: TcpStream,
}

impl PeerMessageStream {
    pub fn connect_to_peer(peer: &Peer) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(format!("{}:{}", peer.ip, peer.port)).unwrap();
        Ok(Self { stream })
    }

    fn create_handshake_message(&self, info_hash: &[u8], peer_id: &[u8]) -> Vec<u8> {
        let mut handshake_message = Vec::new();
        handshake_message.extend_from_slice(&[PSTRLEN]);
        handshake_message.extend_from_slice(b"BitTorrent protocol");
        handshake_message.extend_from_slice(&[0u8; 8]);
        handshake_message.extend_from_slice(info_hash);
        handshake_message.extend_from_slice(peer_id);
        handshake_message
    }
}

impl PeerMessageService for PeerMessageStream {
    fn wait_for_message(&mut self) -> Result<PeerMessage, Box<dyn std::error::Error>> {
        let mut message_length = [0u8; MESSAGE_LENGTH_SIZE];
        self.stream.read_exact(&mut message_length).unwrap();
        let message_length = u32::from_be_bytes(message_length);
        let mut message_id = [0u8; MESSAGE_ID_SIZE];
        self.stream.read_exact(&mut message_id).unwrap();
        let mut payload: Vec<u8> = vec![0; (message_length - 1) as usize];
        self.stream.read_exact(&mut payload).unwrap();

        let msg = PeerMessage {
            id: PeerMessageId::from_u8(message_id[0])?,
            length: message_length,
            payload,
        };
        debug!("message received: {:?}", msg.id);
        Ok(msg)
    }

    fn handshake(
        &mut self,
        info_hash: &[u8],
        peer_id: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let handshake_message = self.create_handshake_message(info_hash, peer_id);
        self.stream.write_all(&handshake_message).unwrap();
        let mut handshake_response = [0u8; HANDSHAKE_LENGTH];
        self.stream.read_exact(&mut handshake_response).unwrap();
        debug!("handshake successful");
        // TODO: fijarse que pasa si el handshake no es correcto
        Ok(())
    }

    fn send_message(&mut self, message: &PeerMessage) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Vec::with_capacity((message.length + 4) as usize);
        bytes.extend_from_slice(&message.length.to_be_bytes());
        bytes.extend_from_slice(&(message.id as u8).to_be_bytes());
        bytes.extend_from_slice(&message.payload);
        self.stream.write_all(&bytes).unwrap();
        debug!("message sent: {:?}", message);
        Ok(())
    }
}

pub struct PeerMessageStreamMock {
    pub counter: u32,
    pub file: Vec<u8>,
    pub block_size: u32,
}

impl PeerMessageService for PeerMessageStreamMock {
    fn wait_for_message(&mut self) -> Result<PeerMessage, Box<dyn std::error::Error>> {
        let msg = PeerMessage::piece(
            0,
            self.counter * self.block_size,
            self.file[(self.counter * self.block_size) as usize
                ..(self.block_size + self.counter * self.block_size) as usize]
                .to_vec(),
        );
        self.counter += 1;
        Ok(msg)
    }

    fn handshake(
        &mut self,
        _info_hash: &[u8],
        _peer_id: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn send_message(&mut self, _message: &PeerMessage) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub trait PeerMessageService {
    fn handshake(
        &mut self,
        info_hash: &[u8],
        peer_id: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn wait_for_message(&mut self) -> Result<PeerMessage, Box<dyn std::error::Error>>;
    fn send_message(&mut self, message: &PeerMessage) -> Result<(), Box<dyn std::error::Error>>;
}
