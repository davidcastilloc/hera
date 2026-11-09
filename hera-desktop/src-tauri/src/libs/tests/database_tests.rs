use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection, sqlite};
use std::path::PathBuf;

use super::database::DB;
use super::track::Track;

/** ----------------------------------------------------------------------------
 * Test data
 * -------------------------------------------------------------------------- */

fn sample_track_1() -> Track {
    Track {
        id: "1".to_string(),
        path: "/music/artist1/album1/track1.mp3".to_string(),
        title: "Song One".to_string(),
        album: "Album One".to_string(),
        album_artist: "Artist One".to_string(),
        artists: vec!["Artist One".to_string()],
        genres: vec!["Pop".to_string(), "Rock".to_string()],
        year: Some(2023),
        duration: 210,
        track_no: Some(1),
        track_of: Some(10),
        disk_no: Some(1),
        disk_of: Some(1),
        is_compilation: false,
        bpm: Some(124.0),
        camelot: Some("8A".to_string()),
        energy: Some(0.80),
        hera_seal: None,
        hot_cues: Some(vec![]),
    }
}

fn sample_track_2() -> Track {
    Track {
        id: "2".to_string(),
        path: "/music/artist2/album2/track2.mp3".to_string(),
        title: "Song Two".to_string(),
        album: "Album Two".to_string(),
        album_artist: "Artist Two".to_string(),
        artists: vec!["Artist Two".to_string()],
        genres: vec!["Jazz".to_string()],
        year: None,
        duration: 180,
        track_no: None,
        track_of: None,
        disk_no: None,
        disk_of: None,
        is_compilation: false,
        bpm: Some(124.0),
        camelot: Some("8A".to_string()),
        energy: Some(0.80),
        hera_seal: None,
        hot_cues: Some(vec![]),
    }
}

fn sample_track_3() -> Track {
    Track {
        id: "3".to_string(),
        path: "/music/artist3/album3/track3.mp3".to_string(),
        title: "Song Three".to_string(),
        album: "Album Three".to_string(),
        album_artist: "Artist Three".to_string(),
        artists: vec!["Artist Three".to_string(), "Artist Four".to_string()],
        genres: vec!["Hip-Hop".to_string()],
        year: Some(2022),
        duration: 240,
        track_no: Some(3),
        track_of: Some(12),
        disk_no: Some(1),
        disk_of: Some(2),
        is_compilation: false,
        bpm: Some(124.0),
        camelot: Some("8A".to_string()),
        energy: Some(0.80),
        hera_seal: None,
        hot_cues: Some(vec![]),
    }
}

async fn get_test_db() -> DB {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .auto_vacuum(sqlite::SqliteAutoVacuum::Incremental);

    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();

    sqlx::migrate!("src/migrations")
        .run(&mut connection)
        .await
        .expect("Failed to run migrations");

    DB { connection }
}

/** ----------------------------------------------------------------------------
 * Integration Test - Tracks
 * -------------------------------------------------------------------------- */

#[tokio::test]
async fn test_tracks_db() {
    let mut db = get_test_db().await;

    // Table should be empty by default
    let mut all_tracks = db.get_all_tracks().await.unwrap();
    assert!(all_tracks.is_empty());

    // Testing insertion of tracks in the DB
    db.insert_tracks(vec![sample_track_1(), sample_track_2(), sample_track_3()])
        .await
        .unwrap();
    all_tracks = db.get_all_tracks().await.unwrap();
    assert!(all_tracks.len() == 3);

    // Inserting a track with the same ID should fail
    assert!(db.insert_tracks(vec![sample_track_1()]).await.is_err());

    // DB.get_tracks should return the track ordered by IDs, and ignored inexisting tracks
    let mut tracks = db
        .get_tracks(&vec!["3".to_string(), "2".to_string()])
        .await
        .unwrap();
    assert_eq!(tracks, vec![sample_track_3(), sample_track_2()]);

    // Test update
    let mut track_to_update = sample_track_2();
    track_to_update.title = "Song Two Point Five".to_string();
    db.update_track(track_to_update).await.unwrap();
    tracks = db.get_tracks(&vec!["2".to_string()]).await.unwrap();
    assert_eq!(
        tracks,
        vec![Track {
            id: "2".to_string(),
            path: "/music/artist2/album2/track2.mp3".to_string(),
            title: "Song Two Point Five".to_string(),
            album: "Album Two".to_string(),
            album_artist: "Artist Two".to_string(),
            artists: vec!["Artist Two".to_string()],
            genres: vec!["Jazz".to_string()],
            year: None,
            duration: 180,
            track_no: None,
            track_of: None,
            disk_no: None,
            disk_of: None,
            is_compilation: false,
            bpm: Some(124.0),
            camelot: Some("8A".to_string()),
            energy: Some(0.80),
            hera_seal: None,
            hot_cues: Some(vec![]),
        }]
    );

    // Test deletion
    db.remove_tracks(&vec!["2".to_string()]).await.unwrap();
    all_tracks = db.get_all_tracks().await.unwrap();
    assert_eq!(all_tracks, vec![sample_track_1(), sample_track_3()]);
}

/** ----------------------------------------------------------------------------
 * Integration Test - Playlists
 * -------------------------------------------------------------------------- */

#[tokio::test]
async fn test_playlists_db() {
    let mut db = get_test_db().await;

    // Tables should be empty by default
    let mut all_playlists = db.get_all_playlists().await.unwrap();
    assert!(all_playlists.is_empty());

    let all_tracks = db.get_all_tracks().await.unwrap();
    assert!(all_tracks.is_empty());
    db.insert_tracks(vec![sample_track_1(), sample_track_2(), sample_track_3()])
        .await
        .unwrap();

    // Test insertion + order
    let playlist_a = db
        .create_playlist(
            "Playlist A".to_string(),
            vec![
                sample_track_1().id,
                sample_track_3().id,
                sample_track_2().id,
                sample_track_1().id,
            ],
            Some(PathBuf::from("/tmp/playlist_a".to_string())),
        )
        .await
        .unwrap();
    db.create_playlist("Playlist b".to_string(), vec![], None)
        .await
        .unwrap();

    all_playlists = db.get_all_playlists().await.unwrap();
    assert_eq!(
        all_playlists
            .into_iter()
            .map(|playlist| playlist.name)
            .collect::<Vec<String>>(),
        vec!["Playlist A".to_string(), "Playlist b".to_string()]
    );

    // Test tracks update
    db.set_playlist_tracks(
        &playlist_a.id,
        vec![sample_track_3().id, sample_track_2().id],
    )
    .await
    .unwrap();
    let mut playlist = db.get_playlist(&playlist_a.id).await.unwrap();
    assert!(playlist.is_some());
    assert_eq!(
        playlist.unwrap().tracks,
        vec!["3".to_string(), "2".to_string()]
    );

    // Test rename
    db.rename_playlist(&playlist_a.id, "My Playlist".to_string())
        .await
        .unwrap();
    playlist = db.get_playlist(&playlist_a.id).await.unwrap();
    assert!(playlist.is_some());
    assert_eq!(playlist.unwrap().name, "My Playlist".to_string());

    // Test deletion
    db.delete_playlist(&playlist_a.id).await.unwrap();
    all_playlists = db.get_all_playlists().await.unwrap();
    assert_eq!(
        all_playlists
            .into_iter()
            .map(|playlist| playlist.name)
            .collect::<Vec<String>>(),
        vec!["Playlist b".to_string()]
    );
}

#[tokio::test]
async fn test_agent_sessions_and_messages_db() {
    use super::database::AgentStoredMessage;

    let mut db = get_test_db().await;

    assert!(db.list_agent_sessions().await.unwrap().is_empty());

    let s1 = db
        .create_agent_session(
            "Nueva sesión".to_string(),
            "cli_agy".to_string(),
            "auto".to_string(),
        )
        .await
        .unwrap();
    let s2 = db
        .create_agent_session(
            "Warmup Viernes".to_string(),
            "local_fast".to_string(),
            "confirm".to_string(),
        )
        .await
        .unwrap();

    let updated = db
        .update_agent_session_meta(
            &s1.id,
            Some("Set Club 124-128"),
            Some("agy-conv-123"),
            None,
            None,
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.title, "Set Club 124-128");
    assert_eq!(updated.agy_conversation_id.as_deref(), Some("agy-conv-123"));

    let msg_user = AgentStoredMessage {
        id: "msg-1".to_string(),
        session_id: s1.id.clone(),
        role: "user".to_string(),
        content: "Arma una playlist 8A".to_string(),
        attachments_json: r#"[{"kind":"track","label":"Opus"}]"#.to_string(),
        tool_steps_json: "[]".to_string(),
        status: "done".to_string(),
        created_at: 1000,
    };
    db.upsert_agent_message(&msg_user).await.unwrap();

    let mut msg_assistant = AgentStoredMessage {
        id: "msg-2".to_string(),
        session_id: s1.id.clone(),
        role: "assistant".to_string(),
        content: "Creando...".to_string(),
        attachments_json: "[]".to_string(),
        tool_steps_json: r#"[{"id":"step-1","tool_name":"create_playlist","status":"running"}]"#
            .to_string(),
        status: "streaming".to_string(),
        created_at: 1001,
    };
    db.upsert_agent_message(&msg_assistant).await.unwrap();

    msg_assistant.content = "Lista creada y enviada a VirtualDJ.".to_string();
    msg_assistant.status = "done".to_string();
    db.upsert_agent_message(&msg_assistant).await.unwrap();

    let msgs = db.get_agent_messages(&s1.id).await.unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].role, "user");
    assert_eq!(msgs[1].content, "Lista creada y enviada a VirtualDJ.");
    assert_eq!(msgs[1].status, "done");

    let listed = db.list_agent_sessions().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, s1.id);

    db.delete_agent_session(&s1.id).await.unwrap();
    assert!(db.get_agent_messages(&s1.id).await.unwrap().is_empty());
    let remaining = db.list_agent_sessions().await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, s2.id);
}
