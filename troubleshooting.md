## Common Issues


### The SSL certificate is invalid; class=Ssl (16); code=Certificate (-17)
Operating system - Ubuntu 18.04
Author - Dospore

#### Fix
export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt


### `#![feature]` may not be used on the stable release channel
Operating system - Ubuntu 18.04
Author - Dospore

#### Explanation
Since we are using some experimental apis we need to tell rustc to use the unstable toolchain.
Some helpful links
- [Switching betwwen toolchains](https://stackoverflow.com/questions/58226545/how-to-switch-between-rust-toolchains)
- [Rust toolchains](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html)

#### Fix
Create a rust-toolchain file in the root of your project with your desired toolchain as per [Switching betwwen toolchains](https://stackoverflow.com/questions/58226545/how-to-switch-between-rust-toolchains)