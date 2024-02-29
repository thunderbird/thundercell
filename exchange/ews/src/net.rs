/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use reqwest::Body;

const ENDPOINT: &str = "https://outlook.office365.com/EWS/Exchange.asmx";

/// Sends the given request body to Office365 with Basic auth. (Gross.)
pub async fn request<B: Into<Body>>(
    username: &str,
    password: &str,
    body: B,
) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post(ENDPOINT)
        .basic_auth(username, Some(password))
        .body(body)
        .send()
        .await?;

    response.text().await
}
