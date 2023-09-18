# Exchange autodiscover

Here lives a quick prototype for an simple EWS autodiscover implementation.

A few notes about this prototype:

* It uses the [POX (Plain Old XML)](https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/pox-autodiscover-web-service-reference-for-exchange) service. The [SOAP](https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/soap-autodiscover-web-service-reference-for-exchange) one seems to be difficult to make it work correctly and doesn't support Exchange 2007, and both Evolution and Thunderbird's current autodiscover implementation use POX.
* Ideally it should try multiple domains, preferrably simultaneously in order to avoid one request blocking the whole process.
* It doesn't try to identify which protocol it's working with (and just takes the last URL in an `Account`>`Protocol`>`ASUrl` element). In the MVP we might want to be a bit more clever with selecting the protocol of the right type. Evolution seems to only support `EXCH` and `EXPR`, with the latter taking precedence, though I'm not 100% sure why ([ref](https://gitlab.gnome.org/GNOME/evolution-ews/-/blob/52053904fc280289b4d1a2b9fa943fa4b347ec22/src/EWS/common/e-ews-connection.c#L2475-2487)).
* It doesn't support [redirection of address](https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/redirectaddr-pox) and [of URL](https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/redirecturl-pox) - we'll want that.
* It uses `Basic` auth, which is not recommended, not sure if I'll have a go at making it work with OAuth before work starts on the MVP.
* [Here](https://gitlab.gnome.org/GNOME/evolution-ews/-/blob/52053904fc280289b4d1a2b9fa943fa4b347ec22/src/EWS/common/e-ews-connection.c#L2409) is a good starting point to understand how Evolution handles autodiscovery responses.
