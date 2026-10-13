use acoustid::AcoustidClient;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mbid = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example list_by_mbid -- <musicbrainz-recording-id>")?;

    // This endpoint does not require an application key.
    let client = AcoustidClient::new()?;
    let response = client.list_by_mbid(&mbid).await?;

    println!(
        "AcoustID tracks for recording {}:",
        response.mbid.as_deref().unwrap_or(&mbid)
    );
    for track in response.tracks {
        println!("  {}", track.id);
    }

    Ok(())
}
