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
cargo +stable install --locked --git https://github.com/CUB3D/anymount anymount
```

Usage:
```shell
# List/id a file
anymount ls <path>
# Extract a file
anymount ext <path>
# View in gui
anymount browse <path>
# List formats
anymount formats
```

Supported formats:
- zip
- gz
- bzip2
- upx
- protobuf
- linux_zimg
- dtbo
- dtb
- shannon_modem
- ext4
- pem
- der
- mtk_md1img
- hmfs
- qcow2
- abootimg
- ohos
- erofs
- cpio
- tar
- unix_ar
- lpf
- sparse
- lz4
- lzfse
- lzo
- vendorboot
- qcom_ptbl
- tlv
- mx140
- ChromeOS OTA
- mbr
- fbpack
- f2fs
- mtk_hblr
- lzma
- mtk_dbg
- esp32_fw
- update_app
- xz
- rar
- rpm
- allwinner_a10
- vbmeta
- pbzx
- xar
- apple_archive
- fbpt
- dtbh
- img4
- zowiebox
- ftab
- uimage
- bootldr
- 7z

## License
GPL3 (vendored code under original licence)