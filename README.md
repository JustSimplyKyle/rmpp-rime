# Installation
1. Compile librime and its dependencies using the remarkable toolchain; precompiled libraries are provided in the release

2. Copy the following libraries to `/home/root/stuff` on you rmpp tablet.

- libglog.so
- libmarisa.so
- libopencc.so
- librime.so
- libunwind.so
- libyaml-cpp.so
- libglog.so.0
- libmarisa.so.0
- libopencc.so.1.1
- librime.so.1
- libunwind.so.8
- libyaml-cpp.so.0.8

3. Copy test_hashed.qmd to `/home/root/xovi/exthome/qt-resource-rebuilder`

4. Compile `rmpp-rime` using the remarkable toolchain, and copy the binary to `/home/root`

5. Modify `xovi/start` as following
```bash
mkdir -p /etc/systemd/system/xochitl.service.d
cat << END > /etc/systemd/system/xochitl.service.d/xovi.conf
[Service]
Environment="QML_DISABLE_DISK_CACHE=1"
Environment="QML_XHR_ALLOW_FILE_WRITE=1"
Environment="QML_XHR_ALLOW_FILE_READ=1"
Environment="LD_PRELOAD=/home/root/xovi/xovi.so"
Environment="LD_LIBRARY_PATH=/home/root/stuff"
END

systemctl daemon-reload
LD_LIBRARY_PATH=/home/root/stuff /home/root/rime daemon &
systemctl restart xochitl
```

6. Copy the schema files for `iridium_bpmf` to `/home/root/rime-user`(currently you have to change the source code to use other input methods)

7. Copy `bopomofo.json` to `/home/root`, and `keyboard_layout.qrr` to `/home/root/xovi/exthome/qt-resource-rebuilder`
(you can change where you want to put the json)

8. Change your keyboard language to icelandic.

# Possible Improvements
-[] uses an xovi inject to avoid installation hassle
-[] arbitrary input methods
