This is a basic proof-of-concept of making requests to and receiving responses
from Exchange Web Services using a Rust command-line interface.

## Trying It Out

The prototype as-is uses Basic auth to avoid the complexities of OAuth handling.
To avoid hardcoding credentials, you will need to create a `config.toml` file in
the root directory of the prototype (i.e., the directory this README is in). It
should look like the following:

```toml
username = "brendans.face@hotmail.com"
password = "s3anrul3z!"
```

You may then run the following (keeping in mind that there is no filtering or
result limiting implemented, so try it with an inbox without many messages in
it):

```
$ cargo run --example find_item
```

If I'm any good at my job, you should get a list of the messages in your inbox,
with ellipsized item IDs and subject.
