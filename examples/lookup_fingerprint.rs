use acoustid::{AcoustIDClient, Metadata};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let duration = args
        .next()
        .ok_or("usage: cargo run --example lookup_fingerprint -- <duration-seconds> <fingerprint>")?
        .parse::<u32>()?;
    let fingerprint = args.next().ok_or(
        "usage: cargo run --example lookup_fingerprint -- <duration-seconds> <fingerprint>",
    )?;
    let application_key = std::env::var("ACOUSTID_APPLICATION_KEY")?;

    let client = AcoustIDClient::with_application_key(application_key)?;
    let response = client
        .lookup_by_fingerprint(
            duration,
            &fingerprint,
            &[Metadata::Recordings, Metadata::RecordingIds],
        )
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
                "  {} — {} ({})",
                artists,
                recording.title.as_deref().unwrap_or("unknown title"),
                recording.id.as_deref().unwrap_or("no recording ID")
            );
        }
    }

    Ok(())
}
