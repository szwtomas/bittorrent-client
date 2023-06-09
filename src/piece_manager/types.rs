use super::sender::types::PieceManagerSender;
use super::worker::types::PieceManagerWorker;
use crate::peer::Bitfield;
use crate::ui::UIMessageSender;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::mpsc;

type PeerId = Vec<u8>;
type PieceId = u32;

#[derive(Debug)]
pub enum PieceManagerMessage {
    PeerPieces(PeerId, Bitfield),
    SuccessfulDownload(PieceId, PeerId),
    FailedDownload(PieceId, PeerId),
    FailedConnection(PeerId),
    Have(PeerId, PieceId),
    ReaskedTracker(),
    FinishedEstablishingConnections(usize),
}

pub fn new_piece_manager(
    number_of_pieces: u32,
    ui_message_sender: UIMessageSender,
    initial_pieces: Vec<u32>,
) -> (PieceManagerSender, PieceManagerWorker) {
    let (tx, rx) = mpsc::channel();

    // Initialize the peers_per_piece HashMap with empty vectors, only the ones needed to be downloaded
    let mut peers_per_piece = HashMap::new();
    for i in 0..number_of_pieces {
        if !initial_pieces.contains(&i) {
            peers_per_piece.insert(i, Vec::new());
        }
    }

    // Initialize remaining_pieces HashSet with all pieces
    let mut remaining_pieces: HashSet<PieceId> = HashSet::new();
    for i in 0..number_of_pieces {
        if !initial_pieces.contains(&i) {
            remaining_pieces.insert(i);
        }
    }

    (
        PieceManagerSender { sender: tx },
        PieceManagerWorker {
            reciever: rx,
            allowed_peers_to_download_piece: peers_per_piece,
            ui_message_sender,
            is_downloading: false,
            piece_asked_to: HashMap::new(),
            pieces_without_peer: HashSet::new(),
            ready_to_download_pieces: remaining_pieces,
            peer_pieces_to_download_count: HashMap::new(),
            recieved_bitfields: 0,
            established_connections: 0,
            is_asking_tracker: false,
        },
    )
}
