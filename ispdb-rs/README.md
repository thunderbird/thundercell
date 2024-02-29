# ispdb-rs

Playground for [autoconfig](https://github.com/thunderbird/autoconfig) in case we ever need a way to
access the XML from other projects.

Note that `autoconfig` is vendored using `git subtree` and does not form any part of the code, it is
used exclusively for testing/validation.

You may update the vendored test tree using the following command, executed from the root of the repo:

    git subtree pull -P ispdb-rs/autoconfig https://github.com/thunderbird/autoconfig.git master --squash