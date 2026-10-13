use acoustid::{AcoustIDClient, Metadata};

const TRACK_ID: &str = "9ff43b6a-4f16-427c-93c2-92307ca505e0";
const RECORDING_MBID: &str = "4e0d8649-1f89-44f3-91af-4c0dbee81f28";
const FINGERPRINT: &str = include_str!("fixtures/fingerprint.txt");

fn application_key() -> Option<String> {
    match std::env::var("ACOUSTID_APPLICATION_KEY") {
        Ok(key) => Some(key),
        Err(_) => {
            eprintln!("skipping live AcoustID endpoint tests: ACOUSTID_APPLICATION_KEY is not set");
            None
        }
    }
}

#[tokio::test]
async fn live_fingerprint_lookup() {
    let Some(application_key) = application_key() else {
        return;
    };
    let client = AcoustIDClient::with_application_key(application_key).unwrap();
    let fingerprint: String = FINGERPRINT.split_whitespace().collect();
    let response = client
        .lookup_by_fingerprint(641, &fingerprint, &[Metadata::RecordingIds])
        .await
        .expect("fingerprint lookup should succeed");
    assert_eq!(response.status, "ok");
}

#[tokio::test]
async fn live_track_id_lookup() {
    let Some(application_key) = application_key() else {
        return;
    };
    let client = AcoustIDClient::with_application_key(application_key).unwrap();
    let response = client
        .lookup_by_track_id(TRACK_ID, &[Metadata::RecordingIds])
        .await
        .expect("track-ID lookup should succeed");
    assert_eq!(response.status, "ok");
}

#[tokio::test]
async fn live_list_acoustids_by_mbid_without_application_key() {
    let client = AcoustIDClient::new().unwrap();
    let response = client
        .list_by_mbid(RECORDING_MBID)
        .await
        .expect("list-by-MBID should succeed without an application key");
    assert_eq!(response.status, "ok");
    assert!(!response.tracks.is_empty());
}
