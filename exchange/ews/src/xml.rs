use crate::types::{EwsWrite, SOAP_NS_URI, TYPES_NS_URI};

/// Writes a struct as the body of a SOAP request.
pub fn write_request<W: std::io::Write, X: EwsWrite<W>>(
    sink: W,
    body: X,
) -> Result<(), quick_xml::Error> {
    let mut writer = quick_xml::writer::Writer::new(sink);

    writer
        .create_element("soap:Envelope")
        .with_attributes([("xmlns:soap", SOAP_NS_URI), ("xmlns:t", TYPES_NS_URI)])
        .write_inner_content::<_, quick_xml::Error>(|writer| {
            writer
                .create_element("soap:Body")
                .write_inner_content(|writer| body.write(writer))?;

            Ok(())
        })?;

    Ok(())
}
