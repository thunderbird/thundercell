/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Support for the autoconfig XML format

use serde::Deserialize;

#[derive(Debug)]
enum AuthenticationMethod {
    None,
    PasswordCleartext,
    PasswordEncrypted,
    NTLM,
    GSSAPI,
    ClientIPAddress,
    TLSClientCert,
    OAuth2,
    HTTPBasic,
    HTTPDigest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ServerKind {
    POP3,
    IMAP,
    SMTP,
}

#[derive(Debug, Deserialize)]
enum SocketKind {
    /// Unencrypted
    Plain,

    /// SSL3/TLS1
    SSL,

    /// Upgrade to TLS on plain socket
    StartTLS,
}

impl<'de> Deserialize<'de> for AuthenticationMethod {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "none" => Ok(Self::None),
            "password-cleartext" => Ok(Self::PasswordCleartext),
            "password-encrypted" => Ok(Self::PasswordEncrypted),
            "NTLM" => Ok(Self::NTLM),
            "GSSAPI" => Ok(Self::GSSAPI),
            "client-IP-address" => Ok(Self::ClientIPAddress),
            "TLS-client-cert" => Ok(Self::TLSClientCert),
            "OAuth2" => Ok(Self::OAuth2),
            "http-basic" => Ok(Self::HTTPBasic),
            "http-digest" => Ok(Self::HTTPDigest),
            _ => Err(serde::de::Error::custom("unsupported AuthenticationMethod")),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Server {
    #[serde(rename(deserialize = "type"))]
    kind: ServerKind,

    /// Remote hostname
    hostname: String,

    /// Username substitution to apply
    username: String,

    /// Remote port
    port: u16,

    /// Kind of socket in use
    #[serde(rename(deserialize = "socketType"))]
    socket_kind: SocketKind,

    /// Supported authentication methods
    authentication: Vec<AuthenticationMethod>,

    /// Possible restrictions on auth
    restriction: Option<Vec<AuthenticationMethod>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EmailDocumentation {
    url: String,
    #[serde(rename(deserialize = "descr"))]
    description: String,
}

/// Contains the matching domains and connection settings
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EmailProvider {
    /// Unique identity for the provider
    id: String,

    /// Domains serviced by this provider
    #[serde(rename(deserialize = "domain"))]
    domains: Vec<String>,

    /// Primary name within the UI
    display_name: String,

    /// Shortened name for UI purposes
    display_short_name: String,

    /// Links to documentation
    documentation: Vec<EmailDocumentation>,

    incoming_server: Vec<Server>,
    outgoing_server: Vec<Server>,
}

/// Contains OAuth2 negotiation settings
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct OAuth2 {
    /// Token issuing authority
    issuer: String,
}

/// Contains links for the WebMail implementation
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WebMail {}

/// A deserialized autoconfig XML file, containing at minimum
/// an [EmailProvider]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AutoconfigXML {
    /// Mandatory email provider record
    email_provider: EmailProvider,

    /// Optional OAuth2 info
    #[serde(rename(deserialize = "oAuth2"))]
    oauth2: Option<OAuth2>,

    /// Optional WebMail info
    web_mail: Option<WebMail>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::AutoconfigXML;

    #[test]
    fn test_basic() {
        let test_file = include_bytes!("../autoconfig/ispdb/googlemail.com.xml");
        let a: AutoconfigXML = serde_xml_rs::from_reader(Cursor::new(test_file)).unwrap();
        assert_eq!(a.email_provider.id, "googlemail.com");
        assert_eq!(a.email_provider.domains.len(), 4);
        eprintln!("AutoConfigXML: {a:?}");

        let oauth2 = a.oauth2.expect("Require oAuth2 spec");
        assert_eq!(oauth2.issuer, "accounts.google.com");
    }
}
