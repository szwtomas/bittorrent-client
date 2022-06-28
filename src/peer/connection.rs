use super::constants::*;
use super::errors::IPeerMessageServiceError;
use super::errors::PeerConnectionError;
use super::types::*;
use super::utils::*;
use super::Peer;
use crate::metainfo::Metainfo;
use crate::ui::UIMessageSender;
use log::*;

pub struct PeerConnection {
    _am_choking: bool,
    _am_interested: bool,
    peer_choking: bool,
    _peer_interested: bool,
    message_service: Box<dyn IClientPeerMessageService + Send>,
    metainfo: Metainfo,
    client_peer_id: Vec<u8>,
    bitfield: Bitfield,
    peer_id: Vec<u8>,
    peer: Peer,
    ui_message_sender: UIMessageSender,
}

impl PeerConnection {
    pub fn new(
        peer: Peer,
        client_peer_id: &[u8],
        metainfo: &Metainfo,
        message_service: Box<dyn IClientPeerMessageService + Send>,
        ui_message_sender: UIMessageSender,
    ) -> Self {
        Self {
            _am_choking: true,
            _am_interested: true,
            peer_choking: true,
            _peer_interested: false,
            client_peer_id: client_peer_id.to_vec(),
            metainfo: metainfo.clone(),
            message_service,
            bitfield: Bitfield::new(),
            peer_id: peer.peer_id.clone(),
            ui_message_sender,
            peer,
        }
    }
    pub fn get_peer_id(&self) -> Vec<u8> {
        self.peer_id.clone()
    }
    pub fn get_peer_ip(&self) -> String {
        self.peer.ip.clone()
    }

    pub fn get_bitfield(&self) -> Bitfield {
        self.bitfield.clone()
    }

    fn wait_for_message(&mut self) -> Result<PeerMessage, IPeerMessageServiceError> {
        let message = self.message_service.wait_for_message()?;
        match message.id {
            PeerMessageId::Unchoke => {
                self.peer_choking = false;
            }
            PeerMessageId::Bitfield => {
                self.bitfield.set_bitfield(&message.payload);
            }
            PeerMessageId::Have => {}
            PeerMessageId::Piece => {}
            _ => {
                return Err(IPeerMessageServiceError::UnhandledMessage);
            }
        }
        Ok(message)
    }

    fn wait_until_ready(&mut self) -> Result<(), IPeerMessageServiceError> {
        loop {
            self.wait_for_message()?;

            if self.peer_choking && self.bitfield.is_empty() {
                break;
            }
        }
        Ok(())
    }

    // Requests a block of data of some piece (index refers to the index of the piece).
    // Data starts from the offset within the piece, and its size is the length requested.
    // Once a block is recieved, it is checked if it is valid, and if it is, it is returned.
    fn request_block(
        &mut self,
        index: u32,
        begin: u32,
        lenght: u32,
    ) -> Result<Vec<u8>, PeerConnectionError> {
        let _block_count = self.metainfo.info.piece_length / BLOCK_SIZE;

        self.message_service
            .send_message(&PeerMessage::request(index, begin, lenght))?;
        loop {
            let message = self.wait_for_message().map_err(|_| {
                PeerConnectionError::PieceRequestingError("Failed while waiting for message".into())
            })?;

            if message.id == PeerMessageId::Piece {
                if valid_block(&message.payload, index, begin) {
                    let block = message.payload[8..].to_vec();
                    // debug!(
                    //     "block {} of {} received",
                    //     (begin / BLOCK_SIZE) + 1,
                    //     block_count,
                    // );
                    // PeerConnection::draw_ascii_progress_bar((begin / BLOCK_SIZE) + 1, block_count);
                    break Ok(block);
                } else {
                    break Err(PeerConnectionError::PieceRequestingError(
                        "Invalid block received".to_string(),
                    ));
                }
            }
        }
    }

    // Requests a specific piece from the peer.
    // It does it sequentially, by requesting blocks of data, until the whole piece is recieved.
    // Once it is complete, we verify its sha1 hash, and return the piece if it is valid.
    pub fn request_piece(
        &mut self,
        piece_index: u32,
        block_size: u32,
    ) -> Result<Vec<u8>, PeerConnectionError> {
        let mut counter = 0;
        let mut piece: Vec<u8> = vec![];
        debug!("requesting piece: {}", piece_index);
        loop {
            let block: Vec<u8> = self.request_block(piece_index, counter, block_size)?;
            piece.extend(block);
            counter += block_size;
            if counter >= self.metainfo.info.piece_length {
                if valid_piece(&piece, piece_index, &self.metainfo) {
                    debug!("recieved full valid piece, piece index: {}", piece_index);
                    break Ok(piece);
                } else {
                    break Err(PeerConnectionError::PieceRequestingError(
                        "Invalid piece received".to_string(),
                    ));
                }
            }
        }
    }

    fn _draw_ascii_progress_bar(current_progress: u32, total_blocks: u32) {
        let progress_bar_width = total_blocks;
        let progress_bar_length =
            (current_progress as f32 / total_blocks as f32) * progress_bar_width as f32;
        let progress_bar_length = progress_bar_length as u32;
        let mut progress_bar = String::new();
        for i in 0..progress_bar_length {
            if i == current_progress {
                break;
            }
            progress_bar.push('#');
        }
        for _ in current_progress..progress_bar_width {
            progress_bar.push('-');
        }

        let final_bar = format!("\t\t\t\t\t\t\t[{}]\n\n", progress_bar);
        _print_green(&final_bar);
    }

    //Executes all steps needed to start an active connection with Peer
    pub fn open_connection(&mut self) -> Result<(), PeerConnectionError> {
        self.message_service
            .handshake(&self.metainfo.info_hash, &self.client_peer_id)
            .map_err(|_| {
                IPeerMessageServiceError::PeerHandshakeError("Handshake error".to_string())
            })?;

        self.message_service
            .send_message(&PeerMessage::unchoke())
            .map_err(|_| {
                IPeerMessageServiceError::SendingMessageError(
                    "Error trying to send unchoke message".to_string(),
                )
            })?;

        self.message_service
            .send_message(&PeerMessage::interested())
            .map_err(|_| {
                IPeerMessageServiceError::SendingMessageError(
                    "Error trying to send interested message".to_string(),
                )
            })?;

        self.wait_until_ready()?;
        self.ui_message_sender.send_new_connection();
        Ok(())
    }
}

fn _print_green(text: &str) {
    println!("\x1b[32m{}\x1b[0m", text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metainfo::Info;
    use crate::metainfo::Metainfo;

    #[test]
    fn gets_real_piece() {
        let file = vec![0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut pieces: Vec<Vec<u8>> = Vec::new();
        pieces.push(sha1_of(&file[0..8].to_vec()));
        pieces.push(sha1_of(&file[8..16].to_vec()));

        let metainfo_mock = Metainfo {
            announce: "".to_string(),
            info: Info {
                piece_length: 8,
                pieces: pieces,
                length: 16,
                name: "".to_string(),
            },
            info_hash: vec![],
        };

        let peer_mock = Peer {
            ip: "".to_string(),
            port: 0,
            peer_id: vec![],
        };
        const BLOCK_SIZE: u32 = 2;
        let peer_message_stream_mock = PeerMessageServiceMock {
            counter: 0,
            file: file.clone(),
            block_size: BLOCK_SIZE,
        };
        let mut peer_connection = PeerConnection::new(
            peer_mock,
            &vec![1, 2, 3, 4],
            &metainfo_mock,
            Box::new(peer_message_stream_mock),
            UIMessageSender::no_ui(),
        );

        let piece = peer_connection.request_piece(0, BLOCK_SIZE);
        assert_eq!(file[0..8], piece.unwrap());
    }

    #[test]
    fn gets_invalid_block() {
        let file = vec![0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0];

        let mut pieces: Vec<Vec<u8>> = Vec::new();
        pieces.push(sha1_of(&file[0..8].to_vec()));
        pieces.push(sha1_of(&file[8..16].to_vec()));

        let metainfo_mock = Metainfo {
            announce: "".to_string(),
            info: Info {
                piece_length: 8,
                pieces: pieces,
                length: 16,
                name: "".to_string(),
            },
            info_hash: vec![],
        };

        let peer_mock = Peer {
            ip: "".to_string(),
            port: 0,
            peer_id: vec![],
        };
        const BLOCK_SIZE: u32 = 2;
        let peer_message_stream_mock = PeerMessageServiceMock {
            counter: 0,
            file: file.clone(),
            block_size: BLOCK_SIZE,
        };
        let mut peer_connection = PeerConnection::new(
            peer_mock,
            &vec![1, 2, 3, 4],
            &metainfo_mock,
            Box::new(peer_message_stream_mock),
            UIMessageSender::no_ui(),
        );

        assert!(matches!(
            peer_connection.request_piece(1, BLOCK_SIZE),
            Err(PeerConnectionError::PieceRequestingError(_))
        ));
    }
}
