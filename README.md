# AcoustID Rust

AcoustID Rust is a client library for the [AcoustID API](https://acoustid.org/webservice). Bindings for all lookup and list API endpoints are provided with built-in rate limiting.

## How to use

Add it to your project with:
```bash
cargo add acoustid
```

See the examples in the [examples](examples). The lookup examples require an AcoustID application key in `ACOUSTID_APPLICATION_KEY`; the list-by-MBID example does not.

For example, list AcoustID tracks for a MusicBrainz recording without a key:

```bash
cargo run --example list_by_mbid -- 4e0d8649-1f89-44f3-91af-4c0dbee81f28
```

Fingerprint and track-ID lookups can be run with:

```bash
export ACOUSTID_APPLICATION_KEY="your-application-key"
cargo run --example lookup_fingerprint -- <duration-seconds> <chromaprint-fingerprint>
cargo run --example lookup_track_id -- <acoustid-track-id>
```

## AI Disclosure

This project was created with the assistance of AI tools. While AI was used to help generate code and documentation, all code has been reviewed and tested by human developers to ensure its correctness and functionality.
