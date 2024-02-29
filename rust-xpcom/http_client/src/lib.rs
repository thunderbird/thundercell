/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use cstr::cstr;
use std::os::raw::c_void;
use std::ptr;

use nserror::{nsresult, NS_OK};
use nsstring::{nsACString, nsCString};
use xpcom::interfaces::{
    nsIChannel, nsIContentPolicy, nsIIOService, nsILoadInfo, nsIPrincipal,
    nsIScriptSecurityManager, nsIStreamListener, nsIStringInputStream, nsIUploadChannel2,
};
use xpcom::{create_instance, get_service, getter_addrefs, nsIID, xpcom_method, RefPtr, XpCom};

#[no_mangle]
pub unsafe extern "C" fn nsRustHttpClientConstructor(
    iid: &nsIID,
    result: *mut *mut c_void,
) -> nsresult {
    let service = HttpClient::new();
    service.QueryInterface(iid, result)
}

#[xpcom::xpcom(implement(nsIRustHttpClient), atomic)]
pub struct HttpClient {}

impl HttpClient {
    pub fn new() -> RefPtr<HttpClient> {
        HttpClient::allocate(InitHttpClient {})
    }

    xpcom_method!(
        request => Request(
            method: *const nsACString,
            url: *const nsACString,
            request_body: *const nsACString,
            content_type: *const nsACString,
            listener: *const nsIStreamListener
        )
    );

    fn request(
        &self,
        method: *const nsACString,
        url: *const nsACString,
        request_body: *const nsACString,
        content_type: *const nsACString,
        listener: *const nsIStreamListener,
    ) -> Result<(), nsresult> {
        // Get the nsIIOService service to generate the nsIChannel.
        let iosrv = get_service::<nsIIOService>(cstr!("@mozilla.org/network/io-service;1"))
            .ok_or(nserror::NS_ERROR_FAILURE)?;

        // Get the nsIScriptSecurityManager service to retrieve an nsIPrincipal we can use in
        // NewChannelFromURI.
        let scriptsecmgr =
            get_service::<nsIScriptSecurityManager>(cstr!("@mozilla.org/scriptsecuritymanager;1"))
                .ok_or(nserror::NS_ERROR_FAILURE)?;

        let principal: RefPtr<nsIPrincipal> =
            getter_addrefs(unsafe { |p| scriptsecmgr.GetSystemPrincipal(p) })?;

        // Create a new nsIChannel to send our request.
        let channel: RefPtr<nsIChannel> = getter_addrefs(|p| unsafe {
            iosrv.NewChannel(
                url,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                principal.coerce(),
                ptr::null(),
                nsILoadInfo::SEC_ALLOW_CROSS_ORIGIN_SEC_CONTEXT_IS_NULL,
                nsIContentPolicy::TYPE_OTHER,
                p,
            )
        })?;

        // Only set a stream for the body if one is provided, and the method isn't GET.
        // We're dereferencing a raw pointer in this condition, so the condition itself needs to be unsafe.
        if unsafe { !(*request_body).is_empty() && *method != nsCString::from("GET") } {
            // Create an input stream for the body (if any).
            let body_stream = create_instance::<nsIStringInputStream>(cstr!(
                "@mozilla.org/io/string-input-stream;1"
            ))
            .ok_or(nserror::NS_ERROR_FAILURE)?;

            // Cast the channel as nsIUploadChannel2 so we can set the input stream and the method.
            // It's preferrable to use nsIUploadChannel2 over nsIUploadChannel, since it allows us to define both the
            // body's input stream and the request's method at once.
            let upload_channel = channel
                .query_interface::<nsIUploadChannel2>()
                .ok_or(nserror::NS_ERROR_FAILURE)?;

            unsafe {
                // Set the data for the stream.
                // TODO: Is SetUTF8Data the correct method to use? Its doc says it should be used by JS code,
                //       but it also works pretty nicely for us here - and avoids requiring to faff trying to convert
                //       nsACString into C-strings.
                body_stream.SetUTF8Data(request_body).to_result()?;

                // Set the stream as the channel's upload stream.
                // Note: Here's how we could set the content-type ourself:
                //     let content_type = nsCString::from("application/json");
                //     let content_type: *const nsACString = &*content_type;
                upload_channel
                    .ExplicitSetUploadStream(body_stream.coerce(), content_type, -1, method, false)
                    .to_result()?;
            }
        }

        // Send the request asynchronously.
        unsafe { channel.AsyncOpen(listener).to_result() }
    }
}