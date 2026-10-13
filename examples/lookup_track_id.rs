use acoustid::{AcoustIDClient, Metadata};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let track_id = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example lookup_track_id -- <acoustid-track-id>")?;
    let application_key = std::env::var("ACOUSTID_APPLICATION_KEY")?;

    let client = AcoustIDClient::with_application_key(application_key)?;
    let response = client
        .lookup_by_track_id(&track_id, &[Metadata::Recordings])
        .await?;

    for result in response.results {
        println!(
            "AcoustID: {} (score: {})",
            result.id.as_deref().unwrap_or("unknown"),
            result
                .score
                .map(|score| score.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        );
        for recording in result.recordings {
            let artists = recording
                .artists
                .iter()
                .filter_map(|artist| artist.name.as_deref())
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "  {} — {}",
                artists,
                recording.title.as_deref().unwrap_or("unknown title")
            );
        }
    }

    Ok(())
}
