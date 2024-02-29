/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/// The `net` module is responsible for making requests to the Exchange Web
/// Services API.
pub mod net;

/// The `types` module defines the various data structures used for EWS requests
/// and responses. It also provides serialization and deserialization routines
/// for these types.
pub mod types;

/// The `xml` module provides utilities for processing of XML.
pub mod xml;
