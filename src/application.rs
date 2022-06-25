use crate::application_constants::*;
use crate::application_errors::ApplicationError;
use crate::config::Config;
use crate::http::HttpsService;
use crate::metainfo::Metainfo;
use crate::peer::PeerConnection;
use crate::peer::PeerMessageService;
use crate::peer_connection_manager::PeerConnectionManager;
use crate::piece_manager::new_piece_manager;
// use crate::piece_manager::PieceManager;
use crate::piece_saver::new_piece_saver;

use crate::tracker::TrackerService;
use crate::ui::{UIMessage, UIMessageSender};
use gtk::{self, glib};
use log::*;
use rand::Rng;

pub fn run_with_torrent(
    torrent_path: &str,
    ui_message_sender: Option<glib::Sender<UIMessage>>,
) -> Result<(), ApplicationError> {
    pretty_env_logger::init();
    info!("Starting bittorrent client...");
    let client_peer_id = rand::thread_rng().gen::<[u8; 20]>();
    let config = Config::from_path(CONFIG_PATH)?;
    info!("Read client configuration successfully");
    let metainfo = Metainfo::from_torrent(torrent_path)?;
    info!(
        "Parsed Metainfo (torrent file) successfully. I'll try to download {}",
        metainfo.info.name
    );
    let ui_message_sender = match ui_message_sender {
        Some(sender) => UIMessageSender::with_ui(&metainfo.info.name, sender),
        None => UIMessageSender::no_ui(),
    };
    ui_message_sender.send_metadata(metainfo.clone());
    // std::thread::sleep(std::time::Duration::from_secs(5));
    // ui_message_sender.send_downloaded_piece(&metainfo.info.name);
    let http_service = HttpsService::from_url(&metainfo.announce)?;
    let mut tracker_service = TrackerService::from_metainfo(
        &metainfo,
        config.listen_port,
        &client_peer_id,
        Box::new(http_service),
    );
    info!("Fetching peers from tracker");
    let tracker_response = tracker_service.get_peers()?;
    ui_message_sender.send_initial_peers(tracker_response.peers.len() as u32);
    info!("Fetched peers from Tracker successfully");

    /* *********************************************************************** */

    let (piece_manager_sender, mut piece_manager_worker) =
        new_piece_manager(ui_message_sender.clone());
    let piece_manager_worker_handle = std::thread::spawn(move || {
        let _ = piece_manager_worker.listen();
    });

    let (peer_connection_manager, peer_connection_manager_handle) = PeerConnectionManager::new();

    let (piece_saver_sender, piece_saver_worker) = new_piece_saver(
        piece_manager_sender.clone(),
        metainfo.info.pieces.clone(),
        config.download_path,
    );

    let piece_saver_worker_handle = std::thread::spawn(move || {
        piece_saver_worker.listen().unwrap();
    });

    piece_manager_sender.start(peer_connection_manager.clone());
    peer_connection_manager.start(piece_manager_sender.clone(), piece_saver_sender.clone());

    if let Some(peer) = tracker_response.peers.get(0) {
        info!(
            "Trying to connect to peer {} and download piece {}",
            peer.ip, 0
        );
        let peer_message_stream = PeerMessageService::connect_to_peer(peer)?;
        PeerConnection::new(
            peer,
            &client_peer_id,
            &metainfo,
            Box::new(peer_message_stream),
            ui_message_sender,
        )
        .run()?;
        info!("Finished download of piece {} from peer: {}", 0, peer.ip);
    }

    trace!("Start closing threads");

    piece_manager_sender.stop();
    peer_connection_manager.stop();
    piece_saver_sender.stop();

    piece_manager_worker_handle.join()?;
    peer_connection_manager_handle.join()?;
    piece_saver_worker_handle.join()?;

    info!("Exited Bitorrent client successfully!");
    Ok(())
}
