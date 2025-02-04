/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::io::{BufRead, Write};

use reqwest::{Client, Request, StatusCode};

use xml::{reader, writer};

// The schema for POX autodiscovery requests.
const REQUEST_SCHEMA: &str =
    "http://schemas.microsoft.com/exchange/autodiscover/outlook/requestschema/2006";
// The schema for POX autodiscovery responses.
const RESPONSE_SCHEMA: &str =
    "http://schemas.microsoft.com/exchange/autodiscover/outlook/responseschema/2006a";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Retrieve the user's address. Using an inner scope here isn't necessary,
    // but I wanted to play with scopes a bit.
    let address: String;
    {
        print!("Enter an address: ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        let stdin = std::io::stdin();
        stdin.lock().read_line(&mut line)?;
        // We need to trim the line here to get rid of the final \n, which would
        // otherwise mess up with generating the Basic authentication header.
        address = String::from(line.trim());
    }

    // Build and send the request.
    let client = Client::new();
    let mut req = build_request(&client, &address, None)?;
    let mut res = client.execute(req).await?;

    // If the request requires authorization, prompt the user for a password
    // and try again.
    if res.status() == StatusCode::UNAUTHORIZED {
        println!("Authentication needed.");
        print!("Enter a password: ");
        std::io::stdout().flush()?;
        let password = rpassword::read_password()?;

        req = build_request(&client, &address, Some(password))?;
        res = client.execute(req).await?;
    }

    // Check the response's status.
    let status = res.status();
    if status == 200 {
        // Request successful: extract the EWS endpoint URL from the body.
        let res_txt = res.text().await?;
        println!(
            "EWS endpoint URL: {}",
            get_url_from_autodiscover_response(res_txt)?
        );
    } else {
        // Request unsuccessful: print the response's code, and optionally its body.
        println!(
            "Failed to retrieve EWS endpoint, server responded with code {}",
            status
        );
        let res_txt = res.text().await?;
        if !res_txt.is_empty() {
            println!("Response body:");
            println!("{}", res_txt);
        }
    }

    Ok(())
}

// Builds an autodiscover request for the given address and (optional) password.
// If a password is given, a Basic authentication header is added to the request.
fn build_request(
    client: &Client,
    address: &String,
    password: Option<String>,
) -> Result<Request, Box<dyn std::error::Error>> {
    // Extract the domain from the request. Note that we don't check that the address
    // is a valid one here (e.g. we don't even check that there's an '@' sign).
    let split = address.split('@');
    let domain = split.last().ok_or("invalid address")?;

    // Start building the request. For now we only try autodiscover.{domain}, but we
    // should also try the domain itself as well as an SRV record lookup.
    let autodiscover_url = format!(
        "https://autodiscover.{}/autodiscover/autodiscover.xml",
        domain
    );
    let request_body = generate_autodiscover_request_body(address)?;
    let mut req = client
        .post(autodiscover_url)
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(request_body);

    if password.is_some() {
        // If a password is provided, add Basic authentication.
        req = req.basic_auth(address, password);
    }

    // Build the request.
    Ok(req.build()?)
}

// Generates the body for a POX EWS autodiscover request for the given email address.
// Spec: https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/pox-autodiscover-request-for-exchange
fn generate_autodiscover_request_body(email: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Buffer to use for writing the body.
    let mut buf = Vec::new();

    // `xml-rs` writer. We add indentation to help with readability when
    // debugging, but that's not strictly necessary.
    let mut writer = writer::EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut buf);

    // Write the request's body using `XmlEvent`s.
    let events = vec![
        writer::XmlEvent::from(
            writer::XmlEvent::start_element("Autodiscover").default_ns(REQUEST_SCHEMA),
        ),
        writer::XmlEvent::from(writer::XmlEvent::start_element("Request")),
        writer::XmlEvent::from(writer::XmlEvent::start_element("EMailAddress")),
        writer::XmlEvent::characters(email),
        writer::XmlEvent::from(writer::XmlEvent::end_element()),
        writer::XmlEvent::from(writer::XmlEvent::start_element("AcceptableResponseSchema")),
        writer::XmlEvent::characters(RESPONSE_SCHEMA),
        writer::XmlEvent::from(writer::XmlEvent::end_element()),
        writer::XmlEvent::from(writer::XmlEvent::end_element()),
        writer::XmlEvent::from(writer::XmlEvent::end_element()),
    ];

    // Write each event.
    for evt in events {
        writer.write(evt)?;
    }

    // Turn the buffer (which should now contain our complete XML document) into
    // a string.
    Ok(std::str::from_utf8(buf.as_slice())?.to_string())
}

// Parse the response from an autodiscover request and extract the URL of the EWS
// endpoint.
// Spec: https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/pox-autodiscover-response-for-exchange
fn get_url_from_autodiscover_response(res: String) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = String::new();

    // Parse the response.
    let res_buf = res.into_bytes();
    let parser = reader::EventReader::new(res_buf.as_slice());

    // Whether we're currently inside an <Account> element.
    let mut in_account = false;
    // Whether we're currently inside a <Protocol> element that's inside an <Account> element.
    // TODO: We should also check the type of the protocol.
    let mut in_protocol = false;
    // Whether we're currently inside an <ASUrl> element that's inside a <Protocol> element
    // that's inside an <Account> element
    let mut in_as_url = false;

    for e in parser {
        match e {
            Ok(reader::XmlEvent::StartElement { name, .. }) => {
                let tag_name = name.local_name;
                match tag_name.as_str() {
                    "Account" => in_account = true,
                    "Protocol" => {
                        if in_account {
                            in_protocol = true
                        }
                    }
                    "ASUrl" => {
                        if in_protocol {
                            in_as_url = true
                        }
                    }
                    _ => {}
                }
            }
            Ok(reader::XmlEvent::EndElement { name }) => {
                let tag_name = name.local_name;
                match tag_name.as_str() {
                    "Account" => {
                        in_account = false;
                    }
                    "Protocol" => {
                        in_protocol = false;
                    }
                    "ASUrl" => {
                        in_as_url = false;
                    }
                    _ => {}
                }
            }
            Ok(reader::XmlEvent::Characters(text)) => {
                if in_as_url {
                    // If we're in an ASUrl element, then the characters in there
                    // are the URL we're looking for.
                    url = text;
                }
            }
            _ => {}
        }
    }

    Ok(url)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    use xml::namespace;

    // Test that we generate valid bodies for autodiscovery requests.
    #[test]
    fn request_body_is_valid() {
        // Address to test with.
        let address = String::from("sylah@domain.test");
        // The expected depth of each element in the XML document.
        let expected_depths: HashMap<&str, i32> = HashMap::from([
            ("Autodiscover", 0),
            ("Request", 1),
            ("EMailAddress", 2),
            ("AcceptableResponseSchema", 2),
        ]);

        // Generate the body.
        let req_body = generate_autodiscover_request_body(&address)
            .expect("failed to generate a request body");

        // The currend depth in the XML document.
        let mut depth = 0;
        // Whether we're in the EMailAddress element.
        let mut in_address = false;
        // Whether the EMailAddress includes text.
        let mut text_in_address = false;
        // Whether we're in the AcceptableResponseSchema element.
        let mut in_res_schema = false;
        // Whether the AcceptableResponseSchema includes text.
        let mut text_in_res_schema = false;

        // Parse the body.
        let buf = req_body.into_bytes();
        let parser = reader::EventReader::new(buf.as_slice());
        for e in parser {
            match e {
                Ok(reader::XmlEvent::StartElement {
                    name,
                    attributes: _,
                    namespace,
                }) => {
                    // Compare the current depth against the expected depth for this element.
                    let tag_name = name.local_name.as_str();
                    let expected_depth = expected_depths.get(tag_name).unwrap_or(&-1).to_owned();

                    assert_eq!(depth, expected_depth, "Invalid depth for tag {}", tag_name);

                    match tag_name {
                        "Autodiscover" => {
                            // Check that the Autodiscover element has the correct default namespace.
                            let default_ns = namespace
                                .get(namespace::NS_EMPTY_URI)
                                .unwrap_or("missing default namespace for Autodiscover tag");

                            assert_eq!(default_ns, REQUEST_SCHEMA);
                        }
                        "EMailAddress" => {
                            in_address = true;
                        }
                        "AcceptableResponseSchema" => {
                            in_res_schema = true;
                        }
                        _ => {}
                    }

                    // Increase the current depth.
                    depth += 1;
                }
                Ok(reader::XmlEvent::Characters(text)) => {
                    if in_address {
                        // If we're in the EMailAddress element, check that the
                        // element's content is the email address.
                        assert_eq!(text, address);
                        text_in_address = true;
                    }

                    if in_res_schema {
                        // If we're in the AcceptableResponseSchema, check that we're
                        // referring to the correct schema.
                        assert_eq!(text, RESPONSE_SCHEMA);
                        text_in_res_schema = true;
                    }
                }
                Ok(reader::XmlEvent::EndElement { name }) => {
                    // If we were in an element we're checking the content of,
                    // check that it isn't empty, and track that we've left it.
                    // TODO: Check that there isn't more than one element of each type.
                    match name.local_name.as_str() {
                        "EMailAddress" => {
                            assert!(text_in_address);
                            in_address = false;
                        }
                        "AcceptableResponseSchema" => {
                            assert!(text_in_res_schema);
                            in_res_schema = false;
                        }
                        _ => {}
                    }
                    // Decrease the current depth.
                    depth -= 1;
                }
                Err(e) => {
                    panic!("{}", e.to_string())
                }
                _ => {}
            }
        }
    }
}
