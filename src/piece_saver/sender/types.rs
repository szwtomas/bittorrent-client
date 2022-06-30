use crate::piece_saver::types::PieceSaverMessage;
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct PieceSaverSender {
    pub sender: Sender<PieceSaverMessage>,
}

impl PieceSaverSender {
    pub fn stop_saving(&self) {
        let _ = self.sender.send(PieceSaverMessage::StopSaving);
    }

    pub fn validate_and_save_piece(&self, piece_index: u32, piece_bytes: Vec<u8>) {
        let _ = self.sender.send(PieceSaverMessage::ValidateAndSavePiece(
            piece_index,
            piece_bytes,
        ));
    }

    // pub fn stop(&self) {
    //     // Msg not yet implemented by logger
    //     // logger.stop_logging();
    // }
}
