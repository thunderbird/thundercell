/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use xml::writer;

use crate::types::{EwsWrite, SOAP_NS_URI, TYPES_NS_URI};

/// Writes a struct as the body of a SOAP request.
pub fn write_request<W: std::io::Write, X: EwsWrite<W>>(
    sink: W,
    body: X,
) -> Result<(), writer::Error> {
    let mut writer = xml::EventWriter::new(sink);

    writer.write(
        xml::writer::XmlEvent::start_element("soap:Envelope")
            .ns("soap", SOAP_NS_URI)
            .ns("t", TYPES_NS_URI),
    )?;
    writer.write(xml::writer::XmlEvent::start_element("soap:Body"))?;

    body.write(&mut writer)?;

    writer.write(xml::writer::XmlEvent::end_element())?;
    writer.write(xml::writer::XmlEvent::end_element())
}
