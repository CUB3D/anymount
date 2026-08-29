# Anymount
## Extract and mount anything


## Building
Install dependencies
```shell
# debian/ubuntu/etc
apt install libfuse-dev pkg-config
# fedora (ostree)
rpm-ostree install fuse-devel openssl-devel perl-FindBin perl-IPC-Cmd perl-File-Compare perl-File-Copy perl-Time-Piece
```
Installing:
```shell
cargo install --path=.
```

Usage:
```shell
# List/id a file
anymount ls <path>
# Extract a file
anymount ext <path>
# View in gui
anymount browse <path>
```

## License
GPL3 (vendored code under original licence)