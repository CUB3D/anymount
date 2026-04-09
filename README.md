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
```shell
cargo install --path=.
```


## License
GPL3 (vendored code under original licence)