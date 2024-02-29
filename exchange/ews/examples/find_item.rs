/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::fs;

use ews::{
    net::request,
    types::{FindItem, FolderId, ItemShape, Response, SoapEnvelope},
    xml::write_request,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    username: String,
    password: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = fs::read_to_string("config.toml").expect("Unable to read config.toml");
    let config: Config = toml::from_str(&config).expect("Unable to parse config.toml");

    // Construct the `FindItem` operation to list the contents of the inbox.
    // Note that there's no pagination or filtering here, so the response could
    // be a lot of messages.
    let body = FindItem::new(
        ews::types::Traversal::Shallow,
        ItemShape {
            base_shape: ews::types::BaseShape::Default,
        },
        vec![FolderId::DistinguishedFolderId {
            id: "inbox".to_string(),
            change_key: None,
            mailbox: None,
        }],
    );

    // Write the request as bytes.
    let mut body_bytes = Vec::new();
    if let Err(err) = write_request(&mut body_bytes, body) {
        eprintln!("Failed to write request: {err}");
    }

    // Send the request to Office365.
    let response = request(&config.username, &config.password, body_bytes)
        .await
        .expect("Unable to complete request");

    let response: SoapEnvelope = serde_xml_rs::from_str(&response).expect("Unable to parse XML");
    match response.body.contents {
        Response::FindItemResponse(response) => {
            // Print a summary of what we found.
            for message in response.messages() {
                let id_short = message
                    .item_id()
                    .id()
                    .get(0..10)
                    .expect("Huh, thought IDs would be long");
                let change_key_short = message
                    .item_id()
                    .change_key()
                    .get(0..10)
                    .expect("Thought change keys would be short too");

                println!(
                    "{}...:{}...: {}",
                    id_short,
                    change_key_short,
                    message.subject()
                );
            }
        }
        _ => panic!("Could not find FindItemResponse"),
    }
}
