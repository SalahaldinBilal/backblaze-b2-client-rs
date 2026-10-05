# Backblaze B2 Client for Rust

[![Crate](https://img.shields.io/crates/v/backblaze-b2-client.svg)](https://crates.io/crates/backblaze-b2-client)

This is a Rust client library for the Backblaze B2 cloud storage service. It provides a convenient interface for interacting with the B2 API. Relies on Tokio async runtime.

The crate provides a simple client `B2SimpleClient` that is just a mapping with b2 requests, and a normal client `B2Client` that provides helpful utilities
and auto re-auth to make life easier, you can access its inner basic client, currently only file uploads with `create_upload`. 

The crate is still a work in progress, so expect breaking changes between 0.0.X versions.

## Features

- Auto re-auth with Backblaze B2.
- Easy file upload handler.
- Mapped all b2 storage request in simple client.

## Installation

Add the following dependency to your project using the `cargo add` command:

```sh
cargo add backblaze-b2-client
```

## Usage

### File Upload

```rust,no_run
use std::time::Duration;

use backblaze_b2_client::client::B2Client;
use tokio::{fs::File, time::sleep};

#[tokio::main]
async fn main() {
    let client = B2Client::new("your_key_id".into(), "your_application_key".into())
        .await
        .unwrap();

    let file = File::open("path_to_file").await.unwrap();
    let metadata = file.metadata().await.unwrap();

    let upload = client
        .create_upload(
            file,
            "file_name".into(),
            "bucket_id".into(),
            None,
            metadata.len(),
            None,
        )
        .await;

    let progress_upload = upload.clone();
    tokio::spawn(async move {
        // Logs progress to console every half a second
        while !progress_upload.has_stopped() {
            println!(
                "status: {:?}, stats: {:.2}",
                progress_upload.status(),
                progress_upload.stats().current_stats()
            );
            sleep(Duration::from_millis(500)).await;
        }
    });

    // Starts the file upload and waits for it to finish
    let file = upload.start().await.unwrap();

    println!("{:#?}", file);
}
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

For any questions or suggestions, please open an issue on GitHub.